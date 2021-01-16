use std::{
    time::{
        Duration
    }
};
use tokio::{
    sync::{
        oneshot,
    },
    time::{
        timeout
    }
};
use futures::{
    future::{
        FutureExt
    },
    stream::{
        StreamExt,
        Stream,
        TryStream,
        TryStreamExt
    }
};
use reqwest::{
    Client,
    Url
};
use m3u8_rs::{
    playlist::{
        MediaPlaylist,
        MediaSegment
    }
};
use super::{
    error::{
        AppError
    }
};

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

pub type UrlGeneratorResult = Vec<MediaSegment>;

pub fn run_url_generator(http_client: Client, 
                         info_url: Url, 
                         mut stop_receiver: oneshot::Receiver<()>) -> impl TryStream<Ok=UrlGeneratorResult, Error=AppError> {
    let stream = async_stream::try_stream!(
        let mut previous_last_segment = 0;
        loop {
            if stop_receiver.try_recv().is_ok(){
                println!("Stop received");
                break;
            }

            // Оборачиваем целиком запрос в таймаут, так как стандартный из Request не хочет работать
            let load_future = timeout(Duration::from_secs(30), media_segments_for_url(&http_client, &info_url));

            let playlist = load_future.await??;
            // println!("Playlist data: {:#?}", playlist);

            let mut seq = playlist.media_sequence;
            let mut results = vec![];
            for segment in playlist.segments.into_iter() {
                if seq > previous_last_segment{
                    if (previous_last_segment > 0) && (seq > (previous_last_segment+1)) {
                        println!("!!!! SEGMENT SKIPPED !!!!");    
                    }
                    println!("Yield segment");
                    // yield segment;
                    results.push(segment);
                    previous_last_segment = seq;
                }
                seq += 1;
            }
            yield results;

            // TODO: Вариант лучше?
            let sleep_time = playlist.target_duration / 6.0 * 1000.0;
            tokio::time::sleep(std::time::Duration::from_millis(sleep_time as u64)).await;
        }
    );
    stream
}