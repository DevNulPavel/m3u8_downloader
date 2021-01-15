use tokio::{
    task::{
        JoinHandle
    },
    sync::{
        mpsc
    }
};
use bytes::{
    Bytes
};
use crate::{
    error::{
        AppError
    }
};

enum Inner{
    Active{
        join: JoinHandle<Result<(), AppError>>,
        sender: mpsc::Sender<Bytes>
    },
    Complete
}

pub struct DataReceiver {
    inner: Inner
}
impl DataReceiver {
    pub fn new(join: JoinHandle<Result<(), AppError>>, sender: mpsc::Sender<Bytes>) -> DataReceiver{
        DataReceiver{
            inner: Inner::Active{
                join,
                sender
            }
        }
    }
    pub async fn stop_and_wait_finish(mut self) -> Result<(), AppError>{
        let old_status = std::mem::replace(&mut self.inner, Inner::Complete);
        match old_status {
            Inner::Active{sender, join} => {
                drop(sender);
                join.await?
            },
            Inner::Complete => {
                Ok(())
            }
        }
    }
    pub async fn send(&self, data: Bytes) -> Result<(), AppError>{
        match &self.inner {
            Inner::Active{sender, ..} => {
                if !sender.is_closed(){
                    sender.send(data).await?;
                }
                Ok(())
            },
            Inner::Complete => {
                Err(AppError::ReceiverSendError)
            }
        }
    }
}
impl Drop for DataReceiver{
    fn drop(&mut self) {
        match &self.inner {
            Inner::Active{..} => {
                panic!("Data receiver dropped while not finished");
            },
            Inner::Complete => {
            }
        }
    }
}