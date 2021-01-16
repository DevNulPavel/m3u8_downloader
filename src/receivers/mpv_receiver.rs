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
use super::{
    receiver::{
        DataReceiver
    }
};

async fn read_from<O>(input: O, limit: u8, prefix: &str)
where
    O: AsyncRead + Unpin
{
    let mut reader = tokio::io::BufReader::new(input);
    // let mut buffer = String::new();
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
        println!("{}: {}", prefix, text);
        buffer.clear();
    }
    println!("MPV READ EXIT");
}

pub fn start_mpv_receiver() -> DataReceiver {
    // Mpv
    let (sender, mut mpv_receiver) = mpsc::channel::<Bytes>(40);
    let join = spawn(async move{
        // TODO: Разобраться с предварительным кешированием
        let mut child = Command::new("mpv")
            .args(&[
                "--no-osd-bar",
                "--cache=yes",
                "--stream-buffer-size=128MiB",
                "--demuxer-max-bytes=128MiB",
                "--demuxer-max-back-bytes=128MiB",
                "--quiet",
                "--really-quiet",
                // "-loglevel", "panic",
                // "--cache-secs=60",
                "-"
            ])
            .stdin(Stdio::piped())
            // .stdout(Stdio::piped())
            // .stderr(Stdio::piped())
            .spawn()?;
        {
            println!("MPV spawned");
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
                        println!("Send data to mpv");
                        if input.write_all(&data).await.is_err(){
                            break;
                        }
                    }
                }else{
                    println!("MPV no stdin");
                }
                read_abort.abort();
                err_abort.abort();
            };
            let _ = futures::future::join3(read_f, err_f, write_f).await;
        }
        child.kill().await?;
        println!("MPV stopped");

        Ok(())
    });
    DataReceiver::new(join, sender)
}