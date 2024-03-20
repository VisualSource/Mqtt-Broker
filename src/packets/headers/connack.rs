#[derive(Debug, Default)]
pub struct AcknowledgeFlags(u8);

impl AcknowledgeFlags {
    pub fn new(session_present: bool) -> Self {
        Self(session_present as u8)
    }
    #[deprecated]
    pub fn set_flags(&mut self, value: u8) {
        self.0 = value;
    }

    /* pub fn session_present(&self) -> bool {
        self.0 & 0x01 == 1
    }
    pub fn set_session_present(&mut self, value: bool) {
        self.0 = value as u8;
    }*/
    #[deprecated]
    pub fn as_byte(&self) -> u8 {
        self.0
    }
}

impl From<AcknowledgeFlags> for u8 {
    fn from(value: AcknowledgeFlags) -> Self {
        value.0
    }
}

impl From<u8> for AcknowledgeFlags {
    fn from(value: u8) -> Self {
        Self(value)
    }
}
