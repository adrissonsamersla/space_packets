use std::cmp::PartialEq;
use std::io::Cursor;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PktType {
    Telemetry = 0,
    Telecommand = 1,
}

#[derive(Debug)]
pub struct PrimaryHeader {
    pub version_number: u8,
    pub packet_type: PktType,
    pub secondary_header_flag: bool,
    pub apid: u16,
    pub sequence_flags: u8,
    pub sequence_counter: u16,
    pub data_length: u16,
}

impl PrimaryHeader {
    pub fn from_buffer(buf: &[u8]) -> PrimaryHeader {
        let mut cursor = Cursor::new(buf);

        let val = cursor.read_u16::<BigEndian>().unwrap();
        let version_number = get_version_number(val);
        let packet_type = get_packet_type(val);
        let secondary_header_flag = get_secondary_header_flag(val);
        let apid = get_apid(val);

        let val = cursor.read_u16::<BigEndian>().unwrap();
        let sequence_flags = get_sequence_flags(val);
        let sequence_counter = get_sequence_counter(val);

        let val = cursor.read_u16::<BigEndian>().unwrap();
        let data_length = val;

        PrimaryHeader {
            version_number,
            packet_type,
            secondary_header_flag,
            apid,
            sequence_flags,
            sequence_counter,
            data_length,
        }
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(6);
        let mut cursor = Cursor::new(&mut buf);

        let mut val: u16;

        // First 2 bytes
        val = (self.version_number as u16) << 13;
        val |= (self.packet_type as u16) << 12;
        val |= (self.secondary_header_flag as u16) << 11;
        val |= self.apid as u16;
        cursor.write_u16::<BigEndian>(val).unwrap();

        // Next 2 bytes
        val = (self.sequence_flags as u16) << 14;
        val |= self.sequence_counter as u16;
        cursor.write_u16::<BigEndian>(val).unwrap();

        // Final 2 bytes
        cursor.write_u16::<BigEndian>(self.data_length).unwrap();

        buf
    }
}

/// Masks to filter the desired fields in the provided buffer
enum FieldsFilter {
    // First 2 bytes (u16)
    VersionNo = 0xE000,
    PkyType = 0x1000,
    SecHdrFlag = 0x0800,
    Apid = 0x07FF,
    // Next 2 bytes (u16)
    SeqFlags = 0xC000,
    SeqCount = 0x3FFF,
}

fn get_version_number(val: u16) -> u8 {
    let filter = FieldsFilter::VersionNo as u16;
    ((val & filter) >> 13) as u8
}

fn get_packet_type(val: u16) -> PktType {
    let filter = FieldsFilter::PkyType as u16;
    let flag = ((val & filter) >> 12) as u8;
    match flag {
        0 => PktType::Telemetry,
        1 => PktType::Telecommand,
        _ => panic!("The masked value should be 0 or 1"),
    }
}

fn get_secondary_header_flag(val: u16) -> bool {
    let filter = FieldsFilter::SecHdrFlag as u16;
    (val & filter) >> 11 != 0
}

fn get_apid(val: u16) -> u16 {
    let filter = FieldsFilter::Apid as u16;
    val & filter
}

fn get_sequence_flags(val: u16) -> u8 {
    let filter = FieldsFilter::SeqFlags as u16;
    ((val & filter) >> 14) as u8
}

fn get_sequence_counter(val: u16) -> u16 {
    let filter = FieldsFilter::SeqCount as u16;
    val & filter
}

//
// UNIT TESTS
//

#[cfg(test)]
mod test {
    use super::*;

    const SP1_HEADER: [u8; 6] = [0x08, 0x73, 0xC1, 0x23, 0x00, 0x0F];
    const SP2_HEADER: [u8; 6] = [0x17, 0x54, 0xC6, 0x82, 0x00, 0x04];

    #[test]
    fn test_sp1() {
        let pkt = PrimaryHeader::from_buffer(&SP1_HEADER);

        assert_eq!(pkt.version_number, 0);
        assert_eq!(pkt.packet_type, PktType::Telemetry);
        assert_eq!(pkt.secondary_header_flag, true);
        assert_eq!(pkt.apid, 0x0073);
        assert_eq!(pkt.sequence_flags, 0x03);
        assert_eq!(pkt.sequence_counter, 0x0123);
        assert_eq!(pkt.data_length, 0x000F);

        let pkt = PrimaryHeader {
            version_number: 0,
            packet_type: PktType::Telemetry,
            secondary_header_flag: true,
            apid: 0x0073,
            sequence_flags: 0x03,
            sequence_counter: 0x0123,
            data_length: 0x000F,
        };
        let buf = pkt.get_buffer();
        assert_eq!(buf, SP1_HEADER);
    }

    #[test]
    fn test_sp2() {
        let pkt = PrimaryHeader::from_buffer(&SP2_HEADER);

        assert_eq!(pkt.version_number, 0);
        assert_eq!(pkt.packet_type, PktType::Telecommand);
        assert_eq!(pkt.secondary_header_flag, false);
        assert_eq!(pkt.apid, 0x0754);
        assert_eq!(pkt.sequence_flags, 0x03);
        assert_eq!(pkt.sequence_counter, 0x0682);
        assert_eq!(pkt.data_length, 0x0004);

        let pkt = PrimaryHeader {
            version_number: 0,
            packet_type: PktType::Telecommand,
            secondary_header_flag: false,
            apid: 0x0754,
            sequence_flags: 0x03,
            sequence_counter: 0x0682,
            data_length: 0x0004,
        };
        let buf = pkt.get_buffer();
        assert_eq!(buf, SP2_HEADER);
    }
}
