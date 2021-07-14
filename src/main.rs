use std::io;
use std::sync::mpsc::Receiver;
use std::thread;

use anyhow::Result;
use env_logger::Env;
use log::{debug, info};

use space_packets::{Packet, Reader};

fn main() {
    // Setting up the logger
    let env_log = Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env_log).init();

    debug!("Setting up the reader...");
    let (mut reader, receiver) = Reader::new(io::stdin());
    debug!("Done!");

    debug!("Starting the Logger job...");
    let loggin_thread = thread::spawn(move || {
        logging(&receiver).unwrap();
    });

    debug!("Starting the Reader job...");
    let reader_thread = thread::spawn(move || {
        reader.run().unwrap();
    });

    reader_thread.join().unwrap();
    debug!("Reader job stopped!");

    loggin_thread.join().unwrap();
    debug!("Logger job stopped!");
}

fn logging(channel: &Receiver<Packet>) -> Result<()> {
    let mut counter: u64 = 0;
    loop {
        let pkt = match channel.recv() {
            Ok(pkt) => pkt,
            Err(_) => return Ok(()),
        };

        counter += 1;
        info!("{} Packet(s) successfully parsed: {:#?}", counter, pkt);
    }
}
