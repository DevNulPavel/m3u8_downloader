mod app_arguments;
mod error;
mod chunks_url_generator;
mod loading_starter;
mod load_stream_to_bytes;

use tokio::{
    runtime::{
        Builder
    },
    io::{
        AsyncWriteExt
    },
    sync::{
        mpsc,
        oneshot
    },
    fs::{
        File
    },
    task::{
        JoinHandle
    },
    pin,
    spawn,
};
use futures::{
    Stream, 
    StreamExt, 
    TryStream, 
    TryStreamExt, 
};
use bytes::{
    Bytes
};
use reqwest::{
    Client,
    Url
};
use m3u8_rs::{
    playlist::{
        MasterPlaylist,
        VariantStream
    }
};
use self::{
    error::{
        AppError
    },
    chunks_url_generator::{
        run_url_generator
    },
    loading_starter::{
        run_loading_stream
    },
    load_stream_to_bytes::{
        loading_stream_to_bytes
    }
};

fn base_url_from_master_playlist_url(url: Url) -> Result<Url, AppError>{
    let mut base_url = url.clone();
    {
        let mut segments = base_url
            .path_segments_mut()
            .map_err(|_| AppError::EmptyUrlSegmentsError )?;
        segments.pop();
        segments.push("/");
    }
    Ok(base_url)
}

async fn request_master_playlist(http_client: &Client, url: Url) -> Result<MasterPlaylist, AppError>{
    let response = http_client
        .get(url)
        .send()
        .await?
        .bytes()
        .await?;

    let playlist_info = m3u8_rs::parse_master_playlist(&response)?.1;

    Ok(playlist_info)
}

fn select_stream(playlist: MasterPlaylist) -> Result<VariantStream, AppError> {
    // TODO: Интерактивный выбор стрима
    playlist
        .variants
        .into_iter()
        .last()
        .ok_or(AppError::MasterStreamIsEmpty)
}

fn run_interrupt_awaiter() -> oneshot::Receiver<()> {
    let (finish_sender, finish_receiver) = tokio::sync::oneshot::channel();
    spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Ctrc + C wait failed");
        finish_sender.send(())
            .expect("Stof signal send failed");
    });
    finish_receiver
}

pub struct DataReceiver {
    join: JoinHandle<Result<(), AppError>>,
    sender: mpsc::Sender<Bytes>
}
impl DataReceiver {
    pub async fn stop_and_wait_finish(self) -> Result<(), AppError>{
        drop(self.sender);
        self.join.await?
    }
    pub async fn send(&self, data: Bytes) -> Result<(), AppError>{
        self.sender.send(data).await?;
        Ok(())
    }
}

fn start_mpv_process() -> DataReceiver {
    // Mpv
    let (sender, mut mpv_receiver) = mpsc::channel::<Bytes>(40);
    let join = spawn(async move{
        // TODO: Разобраться с предварительным кешированием
        let mut child = tokio::process::Command::new("mpv")
            .args(&[
                "--cache=yes",
                "--stream-buffer-size=256MiB",
                "--demuxer-max-bytes=256MiB",
                "--demuxer-max-back-bytes=256MiB",
                "--cache-secs=60",
                "-"
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::inherit())
            .spawn()?;
        {
            println!("MPV spawned");
            if let Some(ref mut stdin) = child.stdin {
                while let Some(data) = mpv_receiver.recv().await{
                    println!("Send data to mpv");

                    // При закрытии MPV просто прерываем обработку, не считаем за ошибку
                    if stdin.write_all(&data).await.is_err(){
                        break;
                    }
                }
            }else{
                println!("MPV no stdin");
            }
        }
        child.kill().await?;
        println!("MPV stopped");

        Ok(())
    });
    DataReceiver{
        join,
        sender
    }
}

fn start_file_writer() -> DataReceiver {
    let (sender, mut file_receiver) = mpsc::channel::<Bytes>(40);
    let join = spawn(async move{
        let mut file = File::create("result.ts").await?;
        while let Some(data) = file_receiver.recv().await{
            println!("Saved to file");
            file.write_all(&data).await?;
        }
        file.sync_all().await?;
        println!("File write stopped");
        Ok(())
    });

    DataReceiver{
        join,
        sender
    }
}

async fn async_main() -> Result<(), AppError> {
    let url_string = std::env::var("M3U_URL").expect("Playlist url needed");

    let http_client = Client::new();

    // Парсим урл на базовый плейлист
    let master_playlist_url = Url::parse(&url_string)?;
    println!("Main playlist url: {}", master_playlist_url);

    // Отбрасываем ссылку на файли плейлиста и получаем базовый URL запросов
    let base_url = base_url_from_master_playlist_url(master_playlist_url.clone())?;
    println!("Base url: {}", base_url);

    // Получаем информацию о плейлисте
    let master_playlist = request_master_playlist(&http_client, master_playlist_url).await?;
    // println!("{:#?}", master_playlist);

    // Выбираем конкретный тип стрима
    let stream_info = select_stream(master_playlist)?;
    // println!("{:#?}", stream_info);

    // Получаем урл для информации о чанках
    let stream_chunks_url = base_url.join(&stream_info.uri)?;
    println!("Chunks info url: {}", stream_chunks_url);

    // Получаем канал о прерывании работы
    let finish_receiver = run_interrupt_awaiter();
        
    // Цепочка из стримов обработки
    let segments_receiver = run_url_generator(http_client.clone(), stream_chunks_url, finish_receiver);
    let loaders_stream = run_loading_stream(http_client.clone(), base_url, segments_receiver);
    let bytes_stream = loading_stream_to_bytes(loaders_stream);

    // Выдаем в результаты
    let receivers = vec![
        start_mpv_process(),  // MPV
        start_file_writer()   // File
    ];

    pin!(bytes_stream);
    let mut found_error = Ok(());
    while let Some(data) = bytes_stream.next().await{
        // Отлавливаем только ошибки в стримах
        let data = match data{
            Ok(data) => data,
            Err(err) => {
                found_error = Err(err);
                break;
            }
        };

        // Отдаем получателям
        let futures_iter = receivers
            .iter()
            .map(|receiver|{
                receiver.send(data.clone())
            });
        let found_err = futures::future::join_all(futures_iter)
            .await
            .into_iter()
            .find(|res|{
                res.is_err()
            });

        // На ошибку отдачи данных просто прекращаем работу цикла
        if found_err.is_some() {
            break;
        }
    }

    println!("All chunks saved to file");

    // Wait all finish
    for receiver in receivers.into_iter(){
        if let Err(err) = receiver.stop_and_wait_finish().await{
            println!("Receiver stop error: {}", err);
        }
    }

    println!("All receivers finished");

    found_error
}

fn main()  -> Result<(), AppError> {
    let runtime = Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Tokio runtime build failed");

    runtime.block_on(async_main())
}