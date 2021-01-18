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
        let mut total_skipped: i32 = 0;
        let mut timeout_val: i64 = 45;  // Для самого первого чанка делаем значение таймаута выше
        while let Some(data) = loaders_receiver.try_next().await?{
            // Если не смогли получить результат одной футуры в течение 5 сек, 
            // делаем 10 попыток на следующих - иначе все
            // C каждой итерацией уменьшаем значение таймаута для чанка
            timeout_val = std::cmp::max(timeout_val - 5, 5);
            match timeout(Duration::from_secs(timeout_val as u64), data).await {
                Ok(data) => {
                    total_skipped = 0;
                    let data = data??;
                    yield data;
                },
                Err(_) =>{
                    total_skipped += 1;
                    error!("Chunk await timeout: {} total", total_skipped);
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