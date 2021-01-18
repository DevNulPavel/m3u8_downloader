mod url_generator;
mod loading_starter;
mod segments_stream_to_segment;
mod load_stream_to_bytes;

use reqwest::{
    Client
};
use url::{
    Url
};
use futures::{
    TryStream
};
use bytes::{
    Bytes
};
use super::{
    error::{
        AppError
    }
};
use self::{
    url_generator::{
        run_segment_info_generator
    },
    loading_starter::{
        run_loading_stream
    },
    segments_stream_to_segment::{
        segments_vec_to_segment
    },
    load_stream_to_bytes::{
        loading_stream_to_bytes
    }
};

pub fn run_loading(http_client: &Client, 
                   base_url: Url, 
                   stream_chunks_url: Url) -> impl TryStream<Ok=Bytes, Error=AppError> {
    // Цепочка из стримов обработки
    let segments_receiver = run_segment_info_generator(http_client.clone(), stream_chunks_url);
    let media_stream = segments_vec_to_segment(segments_receiver);
    let loaders_stream = run_loading_stream(http_client.clone(), base_url, media_stream);
    let bytes_stream = loading_stream_to_bytes(loaders_stream);
    
    bytes_stream
}