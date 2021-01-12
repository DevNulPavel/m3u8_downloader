mod app_arguments;
mod error;
mod downloader;

use tokio::{
    runtime::{
        Builder
    },
    io::{
        AsyncWrite,
        AsyncWriteExt
    },
    sync::{
        mpsc
    },
    fs::{
        File
    },
    spawn
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

    // println!("{:#?}", chunks_info);

    Ok(chunks_info)
}

fn run_segments_stream(http_client: Client, stream_chunks_url: Url) -> mpsc::Receiver<Result<MediaSegment, AppError>>{
    let (sender, receiver) = mpsc::channel(40);
    spawn(async move {
        let mut previous_last_segment = 0;
        loop {
            let playlist_result = media_segments_for_url(&http_client, &stream_chunks_url).await;
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
    receiver
}

// fn run_files_loading(http_client: Client, urls_stream: mpsc::Receiver<Result<MediaSegment, AppError>>) -> Result<(), AppError>{

// }

async fn async_main() -> Result<(), AppError> {
    println!("Hello, world!");

    let url_string = std::env::var("M3U_URL").expect("Playlist url needed");

    let http_client = Client::new();

    let main_playlist_url = Url::parse(&url_string)?;
    println!("Main playlist url: {}", main_playlist_url);

    let mut base_url = main_playlist_url.clone();
    {
        let mut segments = base_url
            .path_segments_mut()
            .map_err(|_| AppError::EmptyUrlSegmentsError )?;
        segments.pop();
        segments.push("/");
    }
    println!("Base url: {}", base_url);

    let response = http_client
        .get(main_playlist_url)
        .send()
        .await?
        .bytes()
        .await?;

    let playlist_info = m3u8_rs::parse_master_playlist(&response)?.1;
    
    println!("{:#?}", playlist_info);

    let stream_info = select_stream(playlist_info)?;

    println!("{:#?}", stream_info);

    let stream_chunks_url = base_url.join(&stream_info.uri)?;
    println!("Chunks info url: {}", stream_chunks_url);
    
    let mut file = File::create("result.ts").await?;

    let mut receiver = run_segments_stream(http_client.clone(), stream_chunks_url);

    while let Some(message) = receiver.recv().await{
        match message{
            Ok(segment) => {
                let http_client = http_client.clone();
                let loading_url = base_url.join(&segment.uri)?;
                println!("Chunk url: {}", loading_url);

                let data = http_client
                    .get(loading_url)
                    .send()
                    .await?
                    .bytes()
                    .await?;

                file.write_all(&data).await?;
            },
            Err(err) => {
                return Err(err);
            }
        }
    }

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