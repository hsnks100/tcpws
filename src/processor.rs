
use anyhow::Ok;
use tokio::{
    sync::mpsc::{UnboundedSender},
};
pub fn processor(tx: UnboundedSender<Vec<u8>>, v: Vec<u8>) -> anyhow::Result<()> {
    tx.send("hello!!!!!!".as_bytes().to_vec())?;
    Ok(())
}