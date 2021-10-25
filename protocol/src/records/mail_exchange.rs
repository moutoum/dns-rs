use std::time::Duration;

use crate::result::Result;
use crate::seek::Seek;
use crate::ser::{Serialize, Serializer};

#[derive(Debug)]
pub struct MailExchange {
    pub domain: String,
    pub _class: u16,
    pub ttl: Duration,
    pub preference: u16,
    pub exchange: String,
}

impl Serialize for MailExchange {
    fn serialize<S>(&self, serializer: &mut S) -> Result<()>
        where
            S: Serializer + Seek
    {
        serializer.serialize_qname(&self.domain)?;
        serializer.serialize_u16(15)?;
        serializer.serialize_u16(1)?;
        serializer.serialize_u32(self.ttl.as_secs() as u32)?;
        let size_pos = serializer.position();
        serializer.serialize_u16(0)?;
        serializer.serialize_u16(self.preference)?;
        serializer.serialize_qname(&self.exchange)?;
        let payload_size = serializer.position() - size_pos + 2;

        let current_position = serializer.position();
        serializer.seek(size_pos)?;
        serializer.serialize_u16(payload_size as u16)?;
        serializer.seek(current_position)
    }
}