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


pub type MediaResult = Result<MediaSegment, AppError>;

pub fn segments_vec_to_segment<S>(receiver: S) -> impl Stream<Item=MediaResult>
where 
    S: Stream<Item=UrlGeneratorResult>
{
    let stream = try_stream!(
        tokio::pin!(receiver);
        while let Some(message) = receiver.next().await{
            let segments = message?;
            for segment in segments.into_iter(){
                yield segment;
            }
        }
    );
    stream
}