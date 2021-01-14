use futures::{
    stream::{
        Stream,
        StreamExt
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

pub fn loading_stream_to_bytes<S>(loaders_receiver: S) -> impl Stream<Item=BytesResult>
where 
    S: Stream<Item=LoadingResult>
{
    let stream = try_stream!(
        tokio::pin!(loaders_receiver);
        while let Some(message) = loaders_receiver.next().await{
            let join = message?.await?;
            let data = join?;
            yield data;
        }
    );
    stream
}