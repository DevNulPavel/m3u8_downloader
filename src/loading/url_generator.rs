use std::{
    time::{
        Duration
    }
};
use tokio::{
    time::{
        timeout
    }
};
use futures::{
    stream::{
        TryStream
    }
};
use reqwest::{
    Client,
    Url
};
use log::{
    debug,
    error,
    trace,
    warn
};
use m3u8_rs::{
    playlist::{
        MediaPlaylist,
        MediaSegment
    }
};
use crate::{
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

pub type SegmentInfoGeneratorResult = Vec<MediaSegment>;

pub fn run_segment_info_generator(http_client: Client, 
                         info_url: Url) -> impl TryStream<Ok=SegmentInfoGeneratorResult, Error=AppError> {
    let stream = async_stream::try_stream!(
        let mut previous_last_segment = 0;
        loop {
            trace!("Request chunks info");

            // Оборачиваем целиком запрос в таймаут, так как стандартный из Request не хочет работать
            let load_future = timeout(Duration::from_secs(20), media_segments_for_url(&http_client, &info_url));

            // В случае таймаута - обрываем стриминг
            let playlist = match load_future.await{
                Ok(playlist) => playlist?,
                Err(_) => {
                    error!("Playlist chunks info timeout");        
                    break;
                }
            };

            // Если у нас нету сегментов - значит стрим закончился
            if playlist.segments.len() == 0 {
                error!("Segments array is empty - stream finished"); 
                break;
            }
            
            trace!("Playlist data: {:#?}", playlist);

            let mut seq = playlist.media_sequence;
            let mut results = vec![];
            for segment in playlist.segments.into_iter() {
                if seq > previous_last_segment{
                    if (previous_last_segment > 0) && (seq > (previous_last_segment+1)) {
                        warn!("SEGMENT INFO SKIPPED");    
                    }
                    debug!("Yield segment");
                    // yield segment;
                    results.push(segment);
                    previous_last_segment = seq;
                }
                seq += 1;
            }
            yield results;

            // TODO: Вариант лучше?
            let sleep_time = playlist.target_duration / 10.0 * 1000.0;
            tokio::time::sleep(std::time::Duration::from_millis(sleep_time as u64)).await;
        }
    );
    stream
}