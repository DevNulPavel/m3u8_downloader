//! m3u8_downloader allows to download m3u8 HLS streams by master playlist link.
//! 
//! # Usage example
//! 
//! ```
//! m3u8_downloader --help
//! m3u8_downloader -m "<link to playlist.m3u8 file>"
//! m3u8_downloader -m -q=sel "<link to playlist.m3u8 file>"
//! m3u8_downloader -m -q=1 "<link to playlist.m3u8 file>"
//! m3u8_downloader -m -v=1 "<link to playlist.m3u8 file>"
//! ```



mod app_arguments;
mod error;
mod loading;
mod receivers;
mod logs;

use std::{
    time::{
        Duration
    }
};
use tokio::{
    runtime::{
        Builder
    },
    select,
    pin,
};
use futures::{
    FutureExt,
    StreamExt,
    TryStreamExt
};
use reqwest::{
    Client,
    Url
};
use log::{
    debug,
    info,
    error
};
use m3u8_rs::{
    playlist::{
        MasterPlaylist,
        VariantStream
    }
};
use self::{
    logs::{
        setup_logs
    },
    error::{
        AppError
    },
    app_arguments::{
        parse_arguments,
        Action,
        StreamQuality,
        VerboseLevel
    },
    loading::{
        run_loading
    },
    receivers::{
        start_file_receiver,
        start_mpv_receiver
    }
};


fn base_url_from_master_playlist_url(url: Url) -> Result<Url, AppError>{
    let mut base_url = url.clone();
    {
        let mut segments = base_url
            .path_segments_mut()
            .map_err(|_| AppError::EmptyUrlSegmentsError )?;
        segments.pop();
        segments.push("/");
    }
    Ok(base_url)
}

async fn request_master_playlist(http_client: &Client, url: &Url) -> Result<MasterPlaylist, AppError>{
    let response = http_client
        .get(url.clone())
        .send()
        .await?
        .bytes()
        .await?;

    let playlist_info = m3u8_rs::parse_master_playlist(&response)?.1;

    Ok(playlist_info)
}

fn print_stream_variants(playlist: &MasterPlaylist) {
    playlist
        .variants
        .iter()
        .enumerate()
        .for_each(|(i, val)|{
            match (&val.resolution, &val.frame_rate){
                (Some(res), Some(rate)) => {
                    println!("{}: {} {}fps", i, res, rate);
                },
                (Some(res), None) => {
                    println!("{}: {}", i, res);
                },
                _ => {
                    println!("{}: ?", i);
                }
            }
        });
}

fn select_stream(playlist: MasterPlaylist, quality_type: StreamQuality) -> Result<VariantStream, AppError> {
    if playlist.variants.len() == 0{
        return Err(AppError::MasterStreamIsEmpty);
    }

    match quality_type {
        StreamQuality::Maximum => {
            playlist
                .variants
                .last()
                .cloned() // TODO: Приходится клонировать, так как не получить итем во владение
                .ok_or(AppError::MasterStreamIsEmpty)
        },
        StreamQuality::Specific(index) => {
            playlist
                .variants
                .get(index as usize)
                .cloned() // TODO: Приходится клонировать, так как не получить итем во владение
                .ok_or(AppError::WrongStreamIndex(index))
        },
        StreamQuality::Select => {
            let count = playlist.variants.len() as u8;
            print_stream_variants(&playlist);

            let index = loop {
                let mut text = String::new();
                let length = std::io::stdin().read_line(&mut text)?;
                if length == 0 {
                    println!("Zero length, input value must be from 0 to {}", count-1);
                    continue;
                }
                match text.trim().parse::<u8>(){
                    Ok(val) if (val < count) => {
                        break val; 
                    },
                    _ => {
                        println!("Input value must be from 0 to {}", count-1);
                    }
                }
            };
            playlist
                .variants
                .get(index as usize)
                .cloned()   // TODO: Приходится клонировать, так как не получить итем во владение
                .ok_or(AppError::WrongStreamIndex(index))
        }
    }
}

async fn run_interrupt_awaiter() {
    tokio::signal::ctrl_c()
        .await
        .expect("Ctrc + C wait failed");
    info!("\nStop received, please wait...\n...and wait...\n...and wait again...");
}

async fn run_stream_finish_awaiter(http_client: Client, master_playlist_url: Url) -> Result<(), AppError> {
    // Таймауты на чанках нормально не работают, окончанием стрима можно считать пустой мастер-плейлист
    loop {
        // Получаем информацию о плейлисте
        debug!("Master playlist accessible check");
        match request_master_playlist(&http_client, &master_playlist_url).await{
            Ok(_) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            },
            Err(e) => {
                match e {
                    AppError::MasterStreamIsEmpty => {
                        error!("\nMaster stream is finished :-(");
                        return Ok(());
                    },
                    err @ _ =>{
                        return Err(err);
                    }
                }
            }
        }
    }
}

async fn async_main() -> Result<(), AppError> {
    // TODO: Поддержка просто файла

    let app_arguments = parse_arguments();

    // Logs
    setup_logs(&app_arguments.verbose);

    // TODO: Завершение стрима
    let http_client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("Http client build failed");

    // Парсим урл на базовый плейлист
    let master_playlist_url = Url::parse(&app_arguments.input)?;
    debug!("Main playlist url: {}", master_playlist_url);

    // Отбрасываем ссылку на файли плейлиста и получаем базовый URL запросов
    let base_url = base_url_from_master_playlist_url(master_playlist_url.clone())?;
    debug!("Base url: {}", base_url);

    // Получаем информацию о плейлисте
    let master_playlist = request_master_playlist(&http_client, &master_playlist_url).await?;
    debug!("{:#?}", master_playlist);

    // Если надо лишь отобразить список стримов - просто отображаем стримы
    let download_info = match app_arguments.action {
        Action::List => {
            print_stream_variants(&master_playlist);
            return Ok(());
        },
        Action::Download(info) => info
    };

    // Выбираем конкретный тип стрима
    let stream_info = select_stream(master_playlist, download_info.stream_quality_value)?;
    debug!("{:#?}", stream_info);

    // Получаем урл для информации о чанках
    let stream_chunks_url = base_url.join(&stream_info.uri)?;
    debug!("Chunks info url: {}", stream_chunks_url);
        
    // Цепочка из стримов обработки
    let bytes_stream = run_loading(&http_client, base_url, stream_chunks_url);

    // Выдаем в результаты
    let receivers = if download_info.mpv {
        vec![
            start_mpv_receiver(app_arguments.verbose),   // MPV
            start_file_receiver(download_info.output_file),  // File
        ]
    }else{
        vec![
            start_file_receiver(download_info.output_file),  // File
        ]
    };

    // TODO: Прерывать работу не в начале, а уже в самом конце
    // Получаем футуры о прерывании работы
    let interrupt_f = run_interrupt_awaiter().into_stream();
    pin!(interrupt_f);
    let stream_finish_f = run_stream_finish_awaiter(http_client, master_playlist_url).into_stream();
    pin!(stream_finish_f);

    // Обработка данных и отдача
    let mut found_error = None;
    let bytes_stream = bytes_stream.into_stream();
    pin!(bytes_stream);

    // Главный цикл, прерывается если в канал прилетело завершение
    loop {
        select! {
            msg = interrupt_f.next() => {
                if msg.is_some(){
                    break;
                }
            },
            msg = stream_finish_f.next() => {
                if let Some(res) = msg {
                    if let Err(err) = res {
                        found_error = Some(err);
                    }
                    break;
                }
            },
            stream_msg = bytes_stream.next() => {
                // Если стри пустой - прерывание
                let data = match stream_msg {
                    Some(data) => data,
                    None => {
                        break;
                    }
                };

                // Отлавливаем только ошибки в стримах
                let data = match data{
                    Ok(data) => data,
                    Err(err) => {
                        found_error = Some(err);
                        break;
                    }
                };

                info!("Chunk received: {} kBytes", data.len() / 1024);

                // Отдаем получателям
                let futures_iter = receivers
                    .iter()
                    .map(move |receiver| {
                        receiver.send(data.clone())
                    });
                let found_err = futures::future::join_all(futures_iter)
                    .await
                    .into_iter()
                    .find(|res|{
                        res.is_err()
                    });

                // На ошибку отдачи данных просто прекращаем работу цикла
                if found_err.is_some() {
                    break;
                }
            }
        }        
    }

    info!("Loading loop finished");

    // Wait all finish
    for receiver in receivers.into_iter(){
        receiver.stop_and_wait_finish().await?;
    }

    info!("All data receivers finished");

    match found_error{
        Some(err) => Err(err),
        None => Ok(())
    }
}

fn main()  -> Result<(), AppError> {
    let runtime = Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Tokio runtime build failed");

    runtime.block_on(async_main())
}