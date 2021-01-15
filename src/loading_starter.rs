use std::{
    time::{
        Duration
    }
};
use tokio::{
    task::{
        JoinHandle,
        JoinError
    },
    spawn,
};
use futures::{
    Stream, 
    StreamExt,
    TryStream,
    TryStreamExt
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
use super::{
    error::{
        AppError
    }
};

async fn load_chunk(http_client: Client, url: Url) -> Result<Bytes, AppError>{
    println!("Loading started");

    let data = http_client
        .get(url)
        .timeout(Duration::from_secs(30))
        .send()
        .await?
        .bytes()
        .await?;

    Ok(data)
}

pub type LoadingJoin = JoinHandle<Result<Bytes, AppError>>;

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
            // println!("Chunk url: {}", loading_url);

            let join = spawn(load_chunk(http_client, loading_url));
            
            yield join;
        }
    );
    stream
}