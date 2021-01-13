mod app_arguments;
mod error;
mod downloader;

use std::{
    pin::{
        Pin
    }
};
use tokio::{
    runtime::{
        Builder
    },
    io::{
        AsyncWrite,
        AsyncWriteExt
    },
    sync::{
        mpsc,
        oneshot,
        Notify
    },
    fs::{
        File
    },
    task::{
        spawn_blocking
    },
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
        MediaPlaylist,
        VariantStream,
        MediaSegment
    }
};
use self::{
    error::{
        AppError
    }
};


fn select_stream(playlist: MasterPlaylist) -> Result<VariantStream, AppError> {
    // TODO: Интерактивный выбор стрима
    playlist
        .variants
        .into_iter()
        .last()
        .ok_or(AppError::MasterStreamIsEmpty)
}

async fn media_segments_for_url(http_client: &Client, stream_chunks_url: &Url) -> Result<MediaPlaylist, AppError> {
    let chunks_data = http_client
        .get(stream_chunks_url.as_ref())
        .send()
        .await?
        .bytes()
        .await?;

    let chunks_info = m3u8_rs::parse_media_playlist(&chunks_data)?.1;

    Ok(chunks_info)
}

// fn run_segments_info_stream(http_client: Client, stream_chunks_url: Url, mut stop: oneshot::Receiver<()>) -> mpsc::Receiver<Result<MediaSegment, AppError>>{
    
// }

async fn load_chunk(http_client: Client, url: Url) -> Result<Bytes, AppError>{
    let data = http_client
        .get(url)
        .send()
        .await?
        .bytes()
        .await?;

    Ok(data)
}

fn run_loading_stream<S>(http_client: Client, base_url: Url, mut segments_receiver: S) -> mpsc::Receiver<Result<tokio::task::JoinHandle<Result<Bytes, AppError>>, AppError>> 
where
    S: tokio_stream::Stream + Send + 'static,
    S::Item: Into<Result<MediaSegment, AppError>>  + Send
{
    let (sender, receiver) = mpsc::channel(40);

    spawn(async move{
        tokio::pin!(segments_receiver);
        while let Some(message) = segments_receiver.next().await{
            match message.into() {
                Ok(segment) => {
                    let http_client = http_client.clone();
                    let loading_url = base_url.join(&segment.uri).unwrap(); // TODO: ???
                    println!("Chunk url: {}", loading_url);
    
                    let join = spawn(load_chunk(http_client, loading_url));
    
                    if sender.send(Ok(join)).await.is_err(){
                        break;
                    }
                },
                Err(err) => {
                    if sender.send(Err(err)).await.is_err(){
                        break;
                    }
                }
            }
        }
    });
    
    receiver
}

fn loading_stream_to_bytes(mut loaders_receiver: mpsc::Receiver<Result<tokio::task::JoinHandle<Result<Bytes, AppError>>, AppError>>) -> mpsc::Receiver<Result<Bytes, AppError>> {
    let (sender, receiver) = mpsc::channel(40);
    spawn(async move {
        while let Some(message) = loaders_receiver.recv().await{
            match message {
                Ok(join) => {
                    let data = join.await.expect("Loader join failed"); // TODO: ???
                    if sender.send(data).await.is_err(){
                        break;
                    }
                },
                Err(err) => {
                    if sender.send(Err(err)).await.is_err(){
                        break;
                    }
                }
            }
        }
    });
    receiver
}

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


trait Processor{
    type InputChannel;
    type OutputChannel;
    fn run(self, input: Self::InputChannel) -> Self::OutputChannel;
}

struct Chain<P: Processor>{
    last_channel: P::OutputChannel
}
impl<P: Processor> Chain<P> {
    pub fn new<N, I>(p: N, c: N::InputChannel) -> Chain<N>
    where 
        N: Processor
    {
        let last_channel = p.run(c);
        Chain{
            last_channel
        }
    }

    fn join<N>(self, p: N) -> Chain<N>
    where 
        N: Processor, 
        P::OutputChannel: Into<N::InputChannel>,
        // N::InputChannel: From<P::OutputChannel>,
    {
        let last_channel = p.run(self.last_channel.into());
        Chain{
            last_channel
        }
    }

    pub fn resolve(self) -> P::OutputChannel{
        self.last_channel
    }
}

struct ChunksUrlsActor{
    http_client: Client,
    url: Url,
}
impl ChunksUrlsActor {
    pub fn new(http_client: Client, url: Url) -> Self {
        ChunksUrlsActor{
            http_client,
            url
        }
    }
}
impl Processor for ChunksUrlsActor{
    type InputChannel = oneshot::Receiver<()>;
    type OutputChannel = Pin<Box<dyn Stream<Item=Result<MediaSegment, AppError>> + Send>>;

    fn run(self, mut stop_receiver: Self::InputChannel) -> Self::OutputChannel {
        let stream = async_stream::try_stream!(
            let mut previous_last_segment = 0;
            loop {
                if stop_receiver.try_recv().is_ok(){
                    println!("Stop received");
                    break;
                }
    
                let playlist = media_segments_for_url(&self.http_client, &self.url).await?;
                let mut seq = playlist.media_sequence;
                for segment in playlist.segments.into_iter() {
                    if seq > previous_last_segment{
                        println!("Yield segment");
                        yield segment;
                        previous_last_segment = seq;
                    }
                    seq += 1;
                }

                let sleep_time = playlist.target_duration / 4.0 * 1000.0;
                tokio::time::sleep(std::time::Duration::from_millis(sleep_time as u64)).await;
            }
        );
        Box::pin(stream)

        /*let (sender, receiver) = mpsc::channel(40);
        spawn(async move {
            let mut previous_last_segment = 0;
            loop {
                if stop_receiver.try_recv().is_ok(){
                    println!("Stop reeived");
                    break;
                }
    
                let playlist_result = media_segments_for_url(&self.http_client, &self.url).await;
                match playlist_result {
                    Ok(playlist) => {   
                        let mut seq = playlist.media_sequence;
                        for segment in playlist.segments.into_iter() {
                            if seq > previous_last_segment{
                                if sender.send(Ok(segment)).await.is_err(){
                                    break;
                                }
                                previous_last_segment = seq;
                            }
                            seq += 1;
                        }
    
                        let sleep_time = playlist.target_duration / 4.0 * 1000.0;
                        tokio::time::sleep(std::time::Duration::from_millis(sleep_time as u64)).await;
                    },
                    Err(err) => {
                        if sender.send(Err(err)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });
        receiver*/
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

    // let segments_receiver = Chain::new(ChunksUrlsActor::new(http_client.clone(), stream_chunks_url), finish_receiver)
    //     .join(p)
    //     .resolve();

    // tokio::pin!(segments_receiver);
        
    let segments_receiver = ChunksUrlsActor::new(http_client.clone(), stream_chunks_url).run(finish_receiver);
    let loaders_stream = run_loading_stream(http_client.clone(), base_url, segments_receiver);
    let mut bytes_stream = loading_stream_to_bytes(loaders_stream);

    // Mpv
    let (mpv_sender, mut mpv_receiver) = mpsc::channel::<Bytes>(40);
    spawn(async move{
        let mut child = tokio::process::Command::new("mpv")
            .args(&[
                "--cache=yes",
                "--stream-buffer-size=64MiB",
                "--demuxer-max-bytes=64MiB",
                "--demuxer-max-back-bytes=64MiB",
                "--cache-secs=20",
                "-"
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::inherit())
            .spawn()
            .unwrap();
        {
            println!("MPV spawned");
            if let Some(ref mut stdin) = child.stdin {
                while let Some(data) = mpv_receiver.recv().await{
                    println!("Send data to mpv");
                    stdin.write_all(&data).await.unwrap();
                }
            }else{
                println!("MPV no stdin");
            }
        }
        child.kill().await.unwrap();
        println!("MPV stopped");
    });

    // File
    let (file_sender, mut file_receiver) = mpsc::channel::<Bytes>(40);
    spawn(async move{
        let mut file = File::create("result.ts").await.unwrap();
        while let Some(data) = file_receiver.recv().await{
            println!("Saved to file");
            file.write_all(&data).await.unwrap();
        }
    });

    while let Some(data) = bytes_stream.recv().await{
        match data{
            Ok(data) => {
                let f1 = mpv_sender.send(data.clone());
                let f2 = file_sender.send(data);
                let _ = tokio::join!(f1, f2);
            },
            Err(err) => {
                return Err(err);
            }
        }
    }

    println!("All chunks saved to file");

    Ok(())
}

fn main()  -> Result<(), AppError> {
    let runtime = Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Tokio runtime build failed");

    runtime.block_on(async_main())
}