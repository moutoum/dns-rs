use crate::result::Result;

pub trait Deserializer : Sized {
    fn deserialize_u8(self) -> Result<u8>;
    fn deserialize_u16(self) -> Result<u16>;
    fn deserialize_u32(self) -> Result<u32>;
    fn deserialize_qname(self) -> Result<String>;
}