use std::io::{BufReader, Cursor, ErrorKind, Read, Seek, SeekFrom};
use std::sync::mpsc::{self, Receiver, SyncSender};

use anyhow::{Context, Error, Result};
use byteorder::{BigEndian, ReadBytesExt};

use crate::protocol::Packet;

/// Size of the packet header. Fixed size: 6 bytes.
pub const HEADER_SIZE: usize = 6;

/// Max size of the data field => variable (depends on data_length field).
pub const DATA_MAX_SIZE: usize = 65536;

/// Size of the buffer used by `Reader`.
/// BUFFER_SIZE = primary header length + data field max length.
pub const BUFFER_SIZE: usize = HEADER_SIZE + DATA_MAX_SIZE;

/// Size of the channel to communicate with the reader
const CHANNEL_SIZE: usize = 1024;

/// Custom abstraction of standard `BufReader`
pub struct Reader<R> {
    reader: BufReader<R>,
    header_buf: Vec<u8>,
    data_buf: Vec<u8>,
    channel: SyncSender<Packet>,
}

impl<R: Read + Unpin> Reader<R> {
    pub fn new(src: R) -> (Reader<R>, Receiver<Packet>) {
        let (sender, receiver) = mpsc::sync_channel(CHANNEL_SIZE);
        let reader = BufReader::with_capacity(BUFFER_SIZE, src);

        (
            Reader {
                reader,
                header_buf: Vec::with_capacity(HEADER_SIZE), // known size
                data_buf: Vec::new(),                        // variable size
                channel: sender,
            },
            receiver,
        )
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let should_stop = self.read()?;

            let pkt = self.parse()?;
            self.channel.send(pkt)?;

            if should_stop {
                break;
            }
        }
        Ok(())
    }

    fn read(&mut self) -> Result<bool, Error> {
        // Reading he primary header of the packet (fixed size: 48bits = 6 u8)
        self.header_buf.resize(HEADER_SIZE, 0); // still needs to be populated
        let res = self.reader.read_exact(&mut self.header_buf);
        match res {
            Ok(_) => Ok(false),
            Err(ref err) if err.kind() == ErrorKind::UnexpectedEof => return Ok(true),
            Err(e) => Err(e),
        }
        .with_context(|| format!("Could not read the header of size `{}`", HEADER_SIZE))?;

        // Parsing the header to get the Packet Data Length
        // As specified by the protocol: #octets = PKT_DATA_LENGTH + 1
        let data_len = self.parse_pkt_length() + 1;

        // Reading the data field, which includes the secondary header
        self.data_buf.resize(data_len, 0);
        self.reader
            .read_exact(&mut self.data_buf)
            .with_context(|| format!("Could not read the body of size `{}`", data_len))?;

        Ok(false)
    }

    fn parse(&self) -> Result<Packet, Error> {
        let pkt = Packet::from_buffers(&self.header_buf, &self.data_buf);
        Ok(pkt)
    }

    /// Since the reading was successfull: this method is not expected to panick!
    fn parse_pkt_length(&self) -> usize {
        let mut cursor = Cursor::new(&self.header_buf);
        cursor
            .seek(SeekFrom::Start(4))
            .expect("Fixed size vector: should reach this position");

        cursor
            .read_u16::<BigEndian>()
            .expect("Reading exactly 16 bits: should parse u16") as usize
    }
}

#[cfg(test)]
mod test {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const VALID_SOURCE: [u8; 22] = [
        8, 115, 193, 35, 0, 15, 0, 0, 18, 52, 0, 171, 205, 239, 165, 165, 90, 90, 195, 60, 193, 248,
    ];
    const WRONG_SOURCE: [u8; 8] = [8, 115, 193, 35, 0, 15, 0, 0];

    #[test]
    fn get_correct_buffers() -> TestResult {
        let (mut reader, receiver) = Reader::new(&VALID_SOURCE[..]);
        reader.run()?;

        let pkt = receiver.recv()?;
        let (header, data) = pkt.into_buffers();

        let correct_header: Vec<u8> = vec![8, 115, 193, 35, 0, 15];
        assert_eq!(&header, &correct_header);

        let correct_data: Vec<u8> = vec![
            0, 0, 18, 52, 0, 171, 205, 239, 165, 165, 90, 90, 195, 60, 193, 248,
        ];
        assert_eq!(&data, &correct_data);

        Ok(())
    }

    #[test]
    #[should_panic]
    fn invalid_source() {
        let (mut reader, _) = Reader::new(&WRONG_SOURCE[..]);
        reader.read().unwrap();
    }
}
