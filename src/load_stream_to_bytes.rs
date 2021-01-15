use futures::{
    stream::{
        Stream,
        StreamExt
    }
};
use tokio::{
    sync::{
        mpsc
    }
};
use bytes::{
    Bytes
};
use async_stream::{
    try_stream
};
use super::{
    error::{
        AppError
    },
    loading_starter::{
        LoadingResult
    }
};


pub type BytesResult = Result<Bytes, AppError>;

pub fn loading_stream_to_bytes(loaders_receiver: mpsc::Receiver<LoadingResult>) -> impl Stream<Item=BytesResult>
{
    let stream = try_stream!(
        tokio::pin!(loaders_receiver);
        while let Some(message) = loaders_receiver.recv().await{
            let join = message?.await?;
            let data = join?;
            yield data;
        }
    );
    stream
}