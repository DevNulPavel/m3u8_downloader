use futures::{
    stream::{
        Stream,
        StreamExt,
        TryStream,
        TryStreamExt
    },
    future::{
        Future
    }
};
use bytes::{
    Bytes
};
use tokio::{
    task::{
        JoinHandle,
        JoinError
    }
};
use async_stream::{
    try_stream
};
use super::{
    error::{
        AppError
    },
    loading_starter::{
        LoadingJoin
    }
};

pub fn loading_stream_to_bytes<S>(loaders_receiver: S) -> impl TryStream<Ok=Bytes, Error=AppError>
where 
    S: TryStream<Ok=LoadingJoin, Error=AppError>
{
    let stream = try_stream!(
        // TODO: ???
        // Преобразует стрим футур в стрим результатов
        let loaders_receiver = loaders_receiver
            .into_stream() 
            .map(|join|{
                futures::future::ready(join)
            })
            .buffered(10);
        tokio::pin!(loaders_receiver);
        while let Some(data) = loaders_receiver.try_next().await?{
            let data = data.await??;
            yield data;
        }
    );
    stream
}

/*pub async fn test<S>(loaders_receiver: S) -> Result<(), AppError>
where 
    S: TryStream<Ok=LoadingJoin, Error=AppError>
{
    // Преобразует стрим футур в стрим результатов
    let loaders_receiver = loaders_receiver
        .into_stream()
        .map(|join|{
            futures::future::ready(join)
        })
        .buffered(10);
    tokio::pin!(loaders_receiver);
    while let Some(data) = loaders_receiver.try_next().await?{
        let data = data.await??;
    }
    Ok(())
}*/