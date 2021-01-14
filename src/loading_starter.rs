use tokio::{
    task::{
        JoinHandle
    },
    spawn,
};
use futures::{
    Stream, 
    StreamExt
};
use bytes::{
    Bytes
};
use reqwest::{
    Client,
    Url
};
use async_stream::{
    try_stream
};
use super::{
    error::{
        AppError
    },
    chunks_url_generator::{
        UrlGeneratorResult
    }
};

async fn load_chunk(http_client: Client, url: Url) -> Result<Bytes, AppError>{
    let data = http_client
        .get(url)
        .send()
        .await?
        .bytes()
        .await?;

    Ok(data)
}

pub type LoadingJoin = JoinHandle<Result<Bytes, AppError>>;
pub type LoadingResult = Result<LoadingJoin, AppError>;

pub fn run_loading_stream<S>(http_client: Client, 
                             base_url: Url, 
                             segments_receiver: S) -> impl Stream<Item=LoadingResult> 
where
    S: Stream<Item=UrlGeneratorResult>
{
    let stream = try_stream!(
        tokio::pin!(segments_receiver);
        while let Some(message) = segments_receiver.next().await{
            let segment = message.into()?;
            let http_client = http_client.clone();
            let loading_url = base_url.join(&segment.uri)?;
            println!("Chunk url: {}", loading_url);

            let join = spawn(load_chunk(http_client, loading_url));
            
            yield join;
        }
    );
    
    stream
}