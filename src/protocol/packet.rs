use std::io::{Cursor, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

use super::primary_header::PrimaryHeader;
use super::secondary_header::SecondaryHeader;
use super::user_data_field::UserDataField;

use super::hasher::{self, INITIAL_VALUE};

#[derive(Clone, Debug)]
pub struct Packet {
    pub pri_header: PrimaryHeader,
    pub sec_header: Option<SecondaryHeader>,
    pub user_data: Option<UserDataField>,
    pub checksum: u16,
}

impl Packet {
    pub fn new(
        pri_header: PrimaryHeader,
        sec_header: Option<SecondaryHeader>,
        user_data: Option<UserDataField>,
    ) -> Packet {
        let pri_buf = pri_header.get_buffer();
        let mut checksum = hasher::compute_partial(INITIAL_VALUE, &pri_buf);

        if let Some(header) = &sec_header {
            let sec_buf = header.get_buffer();
            checksum = hasher::compute_partial(checksum, &sec_buf);
        }

        if let Some(data) = &user_data {
            let data_buf = data.get_buffer();
            checksum = hasher::compute_partial(checksum, &data_buf);
        }

        Packet {
            pri_header,
            sec_header,
            user_data,
            checksum,
        }
    }

    pub fn from_buffers(header_buf: &[u8], data_buf: &[u8]) -> Packet {
        let pri_header = PrimaryHeader::from_buffer(header_buf);

        let has_sec_header = pri_header.secondary_header_flag;
        let has_user_data = if has_sec_header {
            data_buf.len() > 10 // 8 bytes de header e 2 bytes de checksum
        } else {
            data_buf.len() > 2 // 2 bytes de checksum
        };

        // The end of the data field: last two bytes are checksum
        let end = data_buf.len() - 2;

        let (sec_header, user_data) = match (has_sec_header, has_user_data) {
            (true, true) => {
                let header = Some(SecondaryHeader::from_buffer(&data_buf[0..8]));
                let data = Some(UserDataField::from_buffer(&data_buf[8..end]));
                (header, data)
            }
            (true, false) => {
                let header = Some(SecondaryHeader::from_buffer(&data_buf[0..8]));
                let data = None;
                (header, data)
            }
            (false, true) => {
                let header = None;
                let data = Some(UserDataField::from_buffer(&data_buf[0..end]));
                (header, data)
            }
            (false, false) => {
                let header = None;
                let data = None;
                (header, data)
            }
        };

        // Validating the given buffers (using checksum)
        let checksum = hasher::compute_partial(INITIAL_VALUE, &header_buf);
        let checksum = hasher::compute_partial(checksum, &data_buf);
        assert_eq!(checksum, 0);

        let mut cursor = Cursor::new(data_buf);
        cursor.seek(SeekFrom::End(-2)).unwrap();
        let checksum = cursor.read_u16::<BigEndian>().unwrap();

        Packet {
            pri_header,
            sec_header,
            user_data,
            checksum,
        }
    }

    pub fn into_buffer(self) -> Vec<u8> {
        // Primary Header
        let mut buf = self.pri_header.get_buffer();

        // (Optional) Secondary Header
        if let Some(header) = self.sec_header {
            buf.append(&mut header.get_buffer());
        };

        // (Optional) Data Field
        if let Some(data) = self.user_data {
            buf.append(&mut data.get_buffer());
        };

        // Checksum
        hasher::append_checksum(&mut buf);

        buf
    }

    pub fn into_buffers(self) -> (Vec<u8>, Vec<u8>) {
        // Primary Header
        let header = self.pri_header.get_buffer();
        let checksum = hasher::compute_partial(INITIAL_VALUE, &header);

        // Data Field
        let mut buf = Vec::new();
        // (Optional) Secondary Header
        if let Some(header) = self.sec_header {
            buf.append(&mut header.get_buffer());
        };

        // (Optional) Data Field
        if let Some(data) = self.user_data {
            buf.append(&mut data.get_buffer());
        };

        // Checksum
        hasher::append_partial_checksum(checksum, &mut buf);

        (header, buf)
    }
}

//
// UNIT TESTS
//

#[cfg(test)]
mod test {
    use super::*;

    use super::super::primary_header::PktType;

    const SP1_HEADER: [u8; 6] = [0x08, 0x73, 0xC1, 0x23, 0x00, 0x0F];
    const SP1_BODY: [u8; 16] = [
        0x00, 0x00, 0x12, 0x34, 0x00, 0xAB, 0xCD, 0xEF, 0xA5, 0xA5, 0x5A, 0x5A, 0xC3, 0x3C, 0xC1,
        0xF8,
    ];

    const SP2_HEADER: [u8; 6] = [0x17, 0x54, 0xC6, 0x82, 0x00, 0x04];
    const SP2_BODY: [u8; 5] = [0x01, 0x02, 0x00, 0x2D, 0xDD];

    #[test]
    fn test_sp1() {
        let pkt = Packet::from_buffers(&SP1_HEADER, &SP1_BODY);

        assert_eq!(pkt.pri_header.version_number, 0);
        assert_eq!(pkt.pri_header.packet_type, PktType::Telemetry);
        assert_eq!(pkt.pri_header.secondary_header_flag, true);
        assert_eq!(pkt.pri_header.apid, 0x0073);
        assert_eq!(pkt.pri_header.sequence_flags, 0x03);
        assert_eq!(pkt.pri_header.sequence_counter, 0x0123);
        assert_eq!(pkt.pri_header.data_length, 0x000F);

        let sec_header = pkt.sec_header.unwrap();
        assert_eq!(sec_header.time_week, 0x00001234);
        assert_eq!(sec_header.time_ms, 0x00ABCDEF);

        let data_field = pkt.user_data.unwrap();
        assert_eq!(data_field.data, [0xA5, 0xA5, 0x5A, 0x5A, 0xC3, 0x3C]);

        assert_eq!(pkt.checksum, 0xC1F8);
    }

    #[test]
    fn test_sp2() {
        let pkt = Packet::from_buffers(&SP2_HEADER, &SP2_BODY);

        assert_eq!(pkt.pri_header.version_number, 0);
        assert_eq!(pkt.pri_header.packet_type, PktType::Telecommand);
        assert_eq!(pkt.pri_header.secondary_header_flag, false);
        assert_eq!(pkt.pri_header.apid, 0x0754);
        assert_eq!(pkt.pri_header.sequence_flags, 0x03);
        assert_eq!(pkt.pri_header.sequence_counter, 0x0682);
        assert_eq!(pkt.pri_header.data_length, 0x0004);

        assert!(pkt.sec_header.is_none());

        let data_field = pkt.user_data.unwrap();
        assert_eq!(data_field.data, [0x01, 0x02, 0x00]);

        assert_eq!(pkt.checksum, 0x2DDD);
    }
}
