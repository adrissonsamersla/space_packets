use std::io::{Cursor, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Clone, Debug)]
pub struct SecondaryHeader {
    pub time_week: u32,
    pub time_ms: u32,
}

impl SecondaryHeader {
    pub fn from_buffer(buf: &[u8]) -> SecondaryHeader {
        let mut cursor = Cursor::new(buf);
        let time_week = cursor.read_u32::<BigEndian>().unwrap();

        cursor.seek(SeekFrom::Start(4)).unwrap(); // skips 4 bytes = 32 bits
        let time_ms = cursor.read_u32::<BigEndian>().unwrap();

        SecondaryHeader { time_week, time_ms }
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8);
        let mut cursor = Cursor::new(&mut buf);

        cursor.write_u32::<BigEndian>(self.time_week).unwrap();
        cursor.write_u32::<BigEndian>(self.time_ms).unwrap();

        buf
    }
}
