use futures::{
    stream::{
        Stream,
        StreamExt,
        TryStream,
        TryStreamExt
    }
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


pub fn segments_vec_to_segment<S>(receiver: S) -> impl TryStream<Ok=MediaSegment, Error=AppError>
where 
    S: TryStream<Ok=Vec<MediaSegment>, Error=AppError>
{
    let stream = try_stream!(
        let receiver = receiver.into_stream(); // TODO: Можно ли избавиться?
        tokio::pin!(receiver);
        while let Some(segments) = receiver.try_next().await? {
            for segment in segments.into_iter(){
                yield segment;
            }
        }
    );
    stream
}