use futures::{
    stream::{
        StreamExt,
        TryStream,
        TryStreamExt
    }
};
use bytes::{
    Bytes
};
use async_stream::{
    try_stream
};
use crate::{
    error::{
        AppError
    }
};
use super::{
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
            .buffered(20); // TODO: ???
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