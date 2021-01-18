use std::{
    time::{
        Instant
    }
};
use tokio::{
    task::{
        JoinHandle,
    },
    spawn,
};
use futures::{
    StreamExt,
    TryStream,
    TryStreamExt
};
use log::{
    debug
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
        MediaSegment
    }
};
use async_stream::{
    try_stream
};
use crate::{
    error::{
        AppError
    }
};

async fn load_chunk(http_client: Client, url: Url) -> Result<Bytes, AppError>{
    debug!("Loading started");

    // TODO: Приоритезация первой загрузки в очереди

    let data = http_client
        .get(url)
        .send()
        .await?
        .bytes()
        .await?;

    Ok(data)
}

pub struct LoadingJoin {
    pub join: JoinHandle<Result<Bytes, AppError>>,
    pub info: MediaSegment,
    pub load_start_time: Instant
}

pub fn run_loading_stream<S>(http_client: Client, 
                             base_url: Url, 
                             segments_receiver: S) -> impl TryStream<Ok=LoadingJoin, Error=AppError> 
where
    S: TryStream<Ok=MediaSegment, Error=AppError>
{
    let stream = try_stream!(
        let segments_receiver = segments_receiver
            .into_stream()  // TODO: Можно ли убрать???
            .map(|v| futures::future::ready(v) )
            .buffered(10); 
        tokio::pin!(segments_receiver);
        while let Some(segment) = segments_receiver.try_next().await?{
            let http_client = http_client.clone();
            let loading_url = base_url.join(&segment.uri)?;
            debug!("Chunk url: {}", loading_url);

            let join = spawn(load_chunk(http_client, loading_url));
            
            yield LoadingJoin{
                join,
                info: segment,
                load_start_time: Instant::now()
            };
        }
    );
    stream
}