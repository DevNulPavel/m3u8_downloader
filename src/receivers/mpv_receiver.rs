use std::{
    process::{
        Stdio
    }
};
use tokio::{
    process::{
        Command,
        Child
    },
    sync::{
        mpsc
    },
    io::{
        AsyncRead,
        AsyncBufReadExt,
        AsyncWriteExt,
        // AsyncReadExt
    },
    spawn
};
use bytes::{
    Bytes
};
use log::{
    debug,
    info,
    error
};
use crate::{
    VerboseLevel
};
use super::{
    receiver::{
        DataReceiver
    },
};

async fn read_from<O>(input: O, limit: u8, prefix: &str)
where
    O: AsyncRead + Unpin
{
    let mut reader = tokio::io::BufReader::new(input);
    let mut buffer = vec![];
    while let Ok(read) = reader.read_until(limit, &mut buffer).await {
        if read == 0 {
            break;
        }
        if read == 1 && buffer[0] == limit {
            buffer.clear();
            continue;
        }
        let text = std::str::from_utf8(&buffer[0..read]).unwrap().trim();
        debug!("{}: {}", prefix, text); // TODO: As error
        buffer.clear();
    }
    debug!("MPV READ EXIT");
}

pub fn start_mpv_receiver(verbose: VerboseLevel) -> DataReceiver {
    // TODO: Передача параметров в MPV извне
    // Mpv
    let (sender, mut mpv_receiver) = mpsc::channel::<Bytes>(40);
    let join = spawn(async move{
        // TODO: Разобраться с предварительным кешированием
        let mut builder = Command::new("mpv");
        builder
            .args(&[
                "--no-osd-bar",
                "--cache=yes",
                "--stream-buffer-size=128MiB",
                "--demuxer-max-bytes=128MiB",
                "--demuxer-max-back-bytes=128MiB",
                // "--cache-secs=60",
            ])
            .stdin(Stdio::piped());
        match verbose{
            VerboseLevel::None | VerboseLevel::Medium => {
                builder
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .arg("--quiet")
                    .arg("--really-quiet");
            },
            VerboseLevel::Max => {
                builder
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
            },
        }
        let mut child = builder.arg("-").spawn()?;
        {
            info!("MPV spawned");
            let Child{stderr, stdin, stdout, ..} = &mut child;
            let (read_f, read_abort) = futures::future::abortable(async move {
                if let Some(ref mut output) = stdout {
                    read_from(output, '\n' as u8, "MPV").await;
                }
            });
            let (err_f, err_abort) = futures::future::abortable(async move {
                if let Some(ref mut error) = stderr {
                    read_from(error, '\r' as u8, "MPV ERR").await;
                }
            });
            let write_f = async move {
                if let Some(ref mut input) = stdin {
                    while let Some(data) = mpv_receiver.recv().await{
                        // При закрытии MPV просто прерываем обработку, не считаем за ошибку
                        if input.write_all(&data).await.is_err(){
                            break;
                        }
                        debug!("MPV received data");
                    }
                }else{
                    error!("MPV no stdin");
                }
                read_abort.abort();
                err_abort.abort();
            };
            let _ = futures::future::join3(read_f, err_f, write_f).await;
        }
        child.kill().await?;
        info!("MPV stopped");

        Ok(())
    });
    DataReceiver::new(join, sender)
}