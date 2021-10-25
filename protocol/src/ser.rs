use crate::result::Result;
use crate::seek::Seek;

pub trait Serializer {
    fn serialize_u8(&mut self, value: u8) -> Result<()>;
    fn serialize_u16(&mut self, value: u16) -> Result<()>;
    fn serialize_u32(&mut self, value: u32) -> Result<()>;
    fn serialize_qname(&mut self, qname: &str) -> Result<()>;
}

pub trait Serialize {
    fn serialize<S>(&self, serializer: &mut S) -> Result<()>
        where
            S: Serializer + Seek;
}
