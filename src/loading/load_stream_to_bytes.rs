use std::{
    time::{
        Duration,
        Instant
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
    error,
    debug
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
        let mut time_start_buffer: i32 = 3000; // Увеличение таймаута для начальных запусков
        while let Some(data) = loaders_receiver.try_next().await?{
            // Если чанк уже грузился очень долго, тогда скипнем его
            let loading_duration_at_now = Instant::now().duration_since(data.load_start_time);
            let chunk_duration = Duration::from_millis((data.info.duration * 1000.0) as u64);
            let timeout_val = match chunk_duration.checked_sub(loading_duration_at_now){
                Some(v) => {
                    v.as_millis() as u64 + time_start_buffer as u64
                },
                None => 0 + time_start_buffer as u64,
            };

            // Постепенно снижаем начальный буффер времени
            time_start_buffer = if (time_start_buffer - 100) > 0 {
                time_start_buffer - 100
            }else{
                0
            };

            debug!("Chunk timeout value: {}, display duration: {}, already loading: {}", 
                timeout_val, 
                chunk_duration.as_millis(),
                loading_duration_at_now.as_millis()
            );

            // Если не смогли получить результат одной футуры в течение таймаута, 
            // делаем еще попыток на следующих чанках - иначе все
            match timeout(Duration::from_millis(timeout_val), data.join).await {
                Ok(data) => {
                    total_skipped = 0;
                    let data = data??;
                    yield data;
                },
                Err(_) =>{
                    total_skipped += 1;
                    error!("Chunk await timeout: {} total repeat", total_skipped);
                    // Количество повторных фейлов по скорости выгрузки
                    if total_skipped > 15 {
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