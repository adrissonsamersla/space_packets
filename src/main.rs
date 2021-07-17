use tokio::io;
use tokio::sync::broadcast::Receiver;
use tokio::task;

use anyhow::Result;
use env_logger::Env;
use log::{debug, info};

use space_packets::{Packet, Reader};

#[tokio::main]
async fn main() {
    // Setting up the logger
    let env_log = Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env_log).init();

    debug!("Setting up the reader...");
    let (mut reader, mut receiver) = Reader::new(io::stdin());
    debug!("Done!");

    debug!("Starting the Logger job...");
    let loggin_thread = task::spawn(async move {
        logging(&mut receiver).await.unwrap();
    });

    debug!("Starting the Reader job...");
    let reader_thread = task::spawn(async move {
        reader.run().await.unwrap();
    });

    reader_thread.await.unwrap();
    debug!("Reader job stopped!");

    loggin_thread.await.unwrap();
    debug!("Logger job stopped!");
}

async fn logging(channel: &mut Receiver<Packet>) -> Result<()> {
    let mut counter: u64 = 0;
    loop {
        let pkt = match channel.recv().await {
            Ok(pkt) => pkt,
            Err(_) => return Ok(()),
        };

        counter += 1;
        info!("{} Packet(s) successfully parsed: {:#?}", counter, pkt);
    }
}
