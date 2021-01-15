use tokio::{
    task::{
        JoinHandle
    },
    sync::{
        mpsc
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
use m3u8_rs::{
    playlist::{
        MediaSegment
    }
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
                             segments_receiver: S) -> (JoinHandle<Result<(), AppError>>, mpsc::Receiver<LoadingResult>)
where
    S: Stream<Item=UrlGeneratorResult> + Send + 'static
{
    let (sender, receiver) = mpsc::channel(10); // TODO: To parameters
    let join = spawn(async move{
        tokio::pin!(segments_receiver);
        while let Some(message) = segments_receiver.next().await{
            let segment: MediaSegment = message?;
            let http_client = http_client.clone();
            let loading_url = base_url.join(&segment.uri)?;
            println!("Chunk url: {}", loading_url);

            let join = spawn(load_chunk(http_client, loading_url));
            
            if sender.send(Ok(join)).await.is_err() {
                break;
            }
        }
        drop(sender);
        Ok(())
    });
    (join, receiver)
}