use std::{
    time::{
        Duration
    }
};
use futures::{
    stream::{
        StreamExt,
        TryStream,
        TryStreamExt
    },
};
use tokio::{
    time::{
        timeout
    }
};
use bytes::{
    Bytes
};
use log::{
    error
};
use async_stream::{
    try_stream
};
use crate::{error::{self, AppError}};
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
        let mut total_skipped = 0;
        while let Some(data) = loaders_receiver.try_next().await?{
            // Если не смогли получить результат одной футуры в течение 3 сек, 
            // делаем 10 попыток на следующих - иначе все
            match timeout(Duration::from_secs(3), data).await {
                Ok(data) => {
                    total_skipped = 0;
                    let data = data??;
                    yield data;
                },
                Err(_) =>{
                    error!("Chunk await timeout: {} total", total_skipped);
                    total_skipped += 1;
                    if total_skipped > 10 {
                        break;
                    }
                }
            }
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