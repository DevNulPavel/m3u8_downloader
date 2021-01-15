mod app_arguments;
mod error;
mod chunks_url_generator;
mod segments_stream_to_segment;
mod loading_starter;
mod load_stream_to_bytes;
mod receivers;

use std::{
    time::{
        Duration
    }
};
use tokio::{
    runtime::{
        Builder
    },
    sync::{
        oneshot
    },
    pin,
    spawn,
};
use futures::{
    StreamExt,
    Stream,
    TryStream,
    TryStreamExt
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
    },
    segments_stream_to_segment::{
        segments_vec_to_segment
    },
    receivers::{
        start_file_receiver,
        start_mpv_receiver
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
        println!("\nStop scheduled, please wait...\nand wait...\nandwait...");
    });
    finish_receiver
}


async fn async_main() -> Result<(), AppError> {
    // TODO: Поддержка просто файла
    let url_string = std::env::var("M3U_URL").expect("Playlist url needed");

    // TODO: Завершение стрима
    let http_client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("Http client build failed");

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
    let media_stream = segments_vec_to_segment(segments_receiver);
    let loaders_stream = run_loading_stream(http_client.clone(), base_url, media_stream);
    let bytes_stream = loading_stream_to_bytes(loaders_stream);

    // Выдаем в результаты
    let receivers = vec![
        start_mpv_receiver(),   // MPV
        start_file_receiver(),  // File
    ];

    // Обработка данных и отдача
    let mut found_error = None;
    let bytes_stream = bytes_stream.into_stream();
    pin!(bytes_stream);
    while let Some(data) = bytes_stream.next().await{
        // Отлавливаем только ошибки в стримах
        let data = match data{
            Ok(data) => data,
            Err(err) => {
                found_error = Some(err);
                break;
            }
        };

        // Отдаем получателям
        let futures_iter = receivers
            .iter()
            .map(move |receiver| {
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

    println!("Loading loop finished");

    // Wait all finish
    for receiver in receivers.into_iter(){
        if let Err(err) = receiver.stop_and_wait_finish().await{
            println!("Receiver stop error: {}", err);
        }
    }

    println!("All receivers finished");

    match found_error{
        Some(err) => Err(err),
        None => Ok(())
    }
}

fn main()  -> Result<(), AppError> {
    let runtime = Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Tokio runtime build failed");

    runtime.block_on(async_main())
}