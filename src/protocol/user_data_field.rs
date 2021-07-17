#[derive(Clone, Debug)]
pub struct UserDataField {
    pub data: Vec<u8>,
}

impl UserDataField {
    pub fn from_buffer(buf: &[u8]) -> UserDataField {
        UserDataField { data: buf.to_vec() }
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        self.data.clone()
    }
}
