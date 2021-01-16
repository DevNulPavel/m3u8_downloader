use chrono::{
    Utc,
    Local,
    DateTime
};
use tokio::{
    sync::{
        mpsc
    },
    io::{
        AsyncWriteExt
    },
    fs::{
        File
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


pub fn start_file_receiver(path: Option<String>) -> DataReceiver {
    let (sender, mut file_receiver) = mpsc::channel::<Bytes>(10);
    let join = spawn(async move{
        let file_path_str = match path {
            Some(path) => {
                path
            },
            None => {
                let now = Local::now();
                format!("{}.ts", now.format("%Y-%m-%d %H:%M:%S")) // TODO: Почему / вместо :
            }
        };

        let mut file = File::create(file_path_str).await?;
        while let Some(data) = file_receiver.recv().await{
            println!("Saved to file");
            file.write_all(&data).await?;
        }
        file.sync_all().await?;
        println!("File write stopped");
        Ok(())
    });

    DataReceiver::new(join, sender)
}
