use std::{
    process::{
        Stdio
    }
};
use tokio::{
    process::{
        Command
    },
    sync::{
        mpsc
    },
    io::{
        AsyncWriteExt
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


pub fn start_mpv_receiver() -> DataReceiver {
    // Mpv
    let (sender, mut mpv_receiver) = mpsc::channel::<Bytes>(40);
    let join = spawn(async move{
        // TODO: Разобраться с предварительным кешированием
        let mut child = Command::new("mpv")
            .args(&[
                "--no-osd-bar",
                "--cache=yes",
                "--stream-buffer-size=256MiB",
                "--demuxer-max-bytes=256MiB",
                "--demuxer-max-back-bytes=256MiB",
                "--cache-secs=60",
                "-"
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .spawn()?;
        {
            println!("MPV spawned");
            if let Some(ref mut stdin) = child.stdin {
                while let Some(data) = mpv_receiver.recv().await{
                    println!("Send data to mpv");

                    // При закрытии MPV просто прерываем обработку, не считаем за ошибку
                    if stdin.write_all(&data).await.is_err(){
                        break;
                    }
                }
            }else{
                println!("MPV no stdin");
            }
        }
        child.kill().await?;
        println!("MPV stopped");

        Ok(())
    });
    DataReceiver::new(join, sender)
}