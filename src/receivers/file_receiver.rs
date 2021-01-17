use chrono::{
    Local
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
use log::{
    debug,
    info
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

        // TODO: Print total data size

        
        let mut file = File::create(&file_path_str).await?;
        info!("Result file '{}' opened", file_path_str);
        drop(file_path_str);

        while let Some(data) = file_receiver.recv().await{
            debug!("Saved to file");
            file.write_all(&data).await?;
        }
        file.sync_all().await?;
        info!("File write stopped");
        Ok(())
    });

    DataReceiver::new(join, sender)
}
