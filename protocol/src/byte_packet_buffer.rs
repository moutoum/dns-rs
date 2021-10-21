use crate::de::Deserializer;
use crate::errors::Error::OutOfRange;
use crate::result::Result;
use crate::seek::Seek;
use crate::ser::Serializer;

const DEFAULT_BUFFER_SIZE: usize = 512;

pub struct BytePacketBuffer {
    buf: [u8; DEFAULT_BUFFER_SIZE],
    pos: usize,
}

impl Serializer for BytePacketBuffer {
    fn serialize_u8(&mut self, value: u8) -> Result<()> {
        if self.pos + 1 > DEFAULT_BUFFER_SIZE {
            return Err(OutOfRange {
                expected: self.pos + 1,
                max: DEFAULT_BUFFER_SIZE,
            });
        }

        self.buf[self.pos] = value;
        self.pos += 1;
        Ok(())
    }

    fn serialize_u16(&mut self, value: u16) -> Result<()> {
        if self.pos + 2 > DEFAULT_BUFFER_SIZE {
            return Err(OutOfRange {
                expected: self.pos + 2,
                max: DEFAULT_BUFFER_SIZE,
            });
        }

        self.buf[self.pos] = (value >> 8) as u8;
        self.buf[self.pos + 1] = value as u8;
        self.pos += 2;
        Ok(())
    }

    fn serialize_u32(&mut self, value: u32) -> Result<()> {
        if self.pos + 4 > DEFAULT_BUFFER_SIZE {
            return Err(OutOfRange {
                expected: self.pos + 4,
                max: DEFAULT_BUFFER_SIZE,
            });
        }

        self.buf[self.pos] = (value >> 24) as u8;
        self.buf[self.pos + 1] = (value >> 16) as u8;
        self.buf[self.pos + 2] = (value >> 8) as u8;
        self.buf[self.pos + 3] = value as u8;
        self.pos += 4;
        Ok(())
    }

    fn serialize_qname(&mut self, qname: &str) -> Result<()> {
        for label in qname.split(".") {
            let len = label.len();
            self.serialize_u8(len as u8)?;

            if self.pos + len > DEFAULT_BUFFER_SIZE {
                return Err(OutOfRange {
                    expected: self.pos + len,
                    max: DEFAULT_BUFFER_SIZE,
                });
            }

            self.buf[self.pos..self.pos + len].copy_from_slice(label.as_bytes());
            self.pos += len;
        }

        self.serialize_u8(0)
    }
}

impl<'a> Deserializer for &'a mut BytePacketBuffer {
    #[inline]
    fn deserialize_u8(self) -> Result<u8> {
        if self.pos > DEFAULT_BUFFER_SIZE {
            return Err(OutOfRange {
                expected: self.pos,
                max: DEFAULT_BUFFER_SIZE,
            });
        }

        let byte = self.buf[self.pos];
        let position = self.position();
        self.seek(position + 1)?;
        Ok(byte)
    }

    #[inline]
    fn deserialize_u16(self) -> Result<u16> {
        let msb = (self.deserialize_u8()? as u16) << 8;
        let lsb = self.deserialize_u8()? as u16;
        Ok(msb | lsb)
    }

    #[inline]
    fn deserialize_u32(self) -> Result<u32> {
        let msb = (self.deserialize_u16()? as u32) << 16;
        let lsb = self.deserialize_u16()? as u32;
        Ok(msb | lsb)
    }

    fn deserialize_qname(self) -> Result<String> {
        let mut out = String::new();
        let mut working_pos = self.position();
        let mut jumped = false;

        // Starting with an empty delimiter to not pushing the first delimiter.
        // The first delimiter corresponds to the last char in  the qname (e.g: "foo.bar.com.").
        let mut delimiter = "";

        loop {
            let len = self.get_u8(working_pos)?;
            working_pos += 1;

            match len {
                // End of qname.
                0 => break,

                // Pointer to a qname in the packet.
                _ if len & 0xC0 == 0xC0 => {
                    if !jumped {
                        self.seek(working_pos + 1)?;
                    }

                    let msb = len as u16 ^ 0xC0;
                    let lsb = self.get_u8(working_pos)? as u16;
                    let offset = (msb << 8) | lsb;
                    working_pos = offset as usize;
                    jumped = true;
                }

                // Normal case where the first byte is the length of the following label.
                _ => {
                    let label = self.get_range(working_pos, len as usize)?;
                    out.push_str(delimiter);
                    out.push_str(&String::from_utf8_lossy(label).to_lowercase());
                    delimiter = ".";
                    working_pos += len as usize;
                }
            }
        }

        if !jumped {
            self.seek(working_pos)?;
        }

        Ok(out)
    }
}

impl Seek for BytePacketBuffer {
    #[inline]
    fn seek(&mut self, pos: usize) -> Result<()> {
        if pos > DEFAULT_BUFFER_SIZE {
            return Err(OutOfRange {
                expected: pos,
                max: DEFAULT_BUFFER_SIZE,
            });
        }

        self.pos = pos;
        Ok(())
    }

    #[inline]
    fn position(&self) -> usize {
        self.pos
    }
}

impl BytePacketBuffer {
    pub fn new() -> BytePacketBuffer {
        BytePacketBuffer {
            buf: [0; DEFAULT_BUFFER_SIZE],
            pos: 0,
        }
    }

    pub fn from_raw_data(data: &[u8]) -> BytePacketBuffer {
        let mut buf = BytePacketBuffer::new();
        let min = DEFAULT_BUFFER_SIZE.min(data.len());
        buf.buf[..min].copy_from_slice(&data[..min]);
        buf
    }

    fn get_u8(&self, pos: usize) -> Result<u8> {
        if pos > DEFAULT_BUFFER_SIZE {
            return Err(OutOfRange { expected: pos, max: DEFAULT_BUFFER_SIZE });
        }

        Ok(self.buf[pos])
    }

    fn get_range(&self, pos: usize, len: usize) -> Result<&[u8]> {
        if pos + len > DEFAULT_BUFFER_SIZE {
            return Err(OutOfRange {
                expected: pos + len,
                max: DEFAULT_BUFFER_SIZE,
            });
        }

        Ok(&self.buf[pos..pos + len])
    }

    pub fn read_u8(&mut self) -> u8 {
        assert!(self.pos < DEFAULT_BUFFER_SIZE, "pos out of range: {:?} >= {:?}", self.pos, DEFAULT_BUFFER_SIZE);
        let c = self.buf[self.pos];
        self.pos += 1;
        c
    }

    pub fn read_n(&mut self, len: usize) -> Vec<u8> {
        assert!(self.pos + len < DEFAULT_BUFFER_SIZE, "pos out of range: {:?} >= {:?}", self.pos + len, DEFAULT_BUFFER_SIZE);
        let out = self.get_range(self.pos, len).unwrap().into();
        let position = self.position();
        self.seek(position + len).unwrap();
        out
    }

    pub fn read_u16(&mut self) -> u16 {
        let first_byte = (self.read_u8() as u16) << 8;
        let second_byte = self.read_u8() as u16;
        first_byte | second_byte
    }

    pub fn read_u32(&mut self) -> u32 {
        let first_byte = (self.read_u8() as u32) << 24;
        let second_byte = (self.read_u8() as u32) << 16;
        let third_byte = (self.read_u8() as u32) << 8;
        let fourth_byte = self.read_u8() as u32;
        first_byte | second_byte | third_byte | fourth_byte
    }

    pub fn read_qname(&mut self) -> String {
        let mut out = String::new();
        let mut delimiter = "";
        let mut pos = self.position();
        let mut jumped = false;

        loop {
            let len = self.get_u8(pos).unwrap();
            pos += 1;

            match len {
                // End of qname.
                0 => break,

                // Pointer to a qname in the packet.
                _ if len & 0xC0 == 0xC0 => {
                    if !jumped {
                        self.seek(pos + 1).unwrap();
                    }

                    let b1 = len as u16 ^ 0xC0;
                    let b2 = self.get_u8(pos).unwrap() as u16;
                    let offset = (b1 << 8) | b2;
                    pos = offset as usize;
                    jumped = true;
                }

                // Normal case where the first byte is the length of the following label.
                _ => {
                    let label = self.get_range(pos, len as usize).unwrap();
                    out.push_str(delimiter);
                    out.push_str(&String::from_utf8_lossy(label).to_lowercase());
                    delimiter = ".";
                    pos += len as usize;
                }
            }
        }

        if !jumped {
            self.seek(pos).unwrap();
        }

        out
    }

    pub fn set_u8(&mut self, pos: usize, value: u8) {
        self.buf[pos] = value;
    }

    pub fn set_u16(&mut self, pos: usize, value: u16) {
        self.buf[pos] = (value >> 8) as u8;
        self.buf[pos + 1] = value as u8;
    }

    pub fn set_u32(&mut self, pos: usize, value: u32) {
        self.buf[pos] = (value >> 24) as u8;
        self.buf[pos + 1] = (value >> 16) as u8;
        self.buf[pos + 2] = (value >> 8) as u8;
        self.buf[pos + 3] = value as u8;
    }

    pub fn bytes(self) -> Vec<u8> {
        (self.buf[..self.pos]).to_vec()
    }
}

#[cfg(test)]
mod test {
    use crate::byte_packet_buffer::BytePacketBuffer;
    use crate::ser::Serializer;

    #[test]
    fn read_u8() {
        let mut buf = BytePacketBuffer::from_raw_data(&[0xDE, 0xAD]);
        assert_eq!(0xDE, buf.read_u8());
        assert_eq!(0xAD, buf.read_u8());
    }

    #[test]
    fn read_u16() {
        let mut buf = BytePacketBuffer::from_raw_data(&[0xDE, 0xAD]);
        assert_eq!(0xDEAD, buf.read_u16());
    }

    #[test]
    fn read_u32() {
        let mut buf = BytePacketBuffer::from_raw_data(&[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(0xDEAD_BEEF, buf.read_u32());
    }

    #[test]
    fn read_qname() {
        let packet: &[u8] = &[
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, // len=6 label="google"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
        ];

        let mut buf = BytePacketBuffer::from_raw_data(packet);
        assert_eq!("www.google.com", buf.read_qname());
    }

    #[test]
    fn read_qname_pointer() {
        let packet: &[u8] = &[
            0xC0, 0x02, // pointer to pos=2
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, // len=6 label="google"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
        ];

        let mut buf = BytePacketBuffer::from_raw_data(packet);
        assert_eq!("www.google.com", buf.read_qname());
    }

    #[test]
    fn read_qnames() {
        let packet: &[u8] = &[
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, // len=6 label="google"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x05, 0x79, 0x61, 0x68, 0x6f, 0x6f, // len=6 label="yahoo"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
        ];

        let mut buf = BytePacketBuffer::from_raw_data(packet);
        assert_eq!("www.google.com", buf.read_qname());
        assert_eq!("www.yahoo.com", buf.read_qname());
    }

    #[test]
    fn read_qname_pointers() {
        let packet: &[u8] = &[
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, // len=6 label="google"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x05, 0x79, 0x61, 0x68, 0x6f, 0x6f, // len=6 label="yahoo"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
            0xC0, 0x10, // Pointer to www.yahoo.com
            0xC0, 0x00 // Pointer to www.google.com
        ];

        let mut buf = BytePacketBuffer::from_raw_data(packet);
        assert_eq!("www.google.com", buf.read_qname());
        assert_eq!("www.yahoo.com", buf.read_qname());
        assert_eq!("www.yahoo.com", buf.read_qname());
        assert_eq!("www.google.com", buf.read_qname());
    }

    #[test]
    fn read_qname_pointer_to_pointer() {
        let packet: &[u8] = &[
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x05, 0x79, 0x61, 0x68, 0x6f, 0x6f, // len=6 label="yahoo"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
            0xC0, 0x00, // Pointer to www.yahoo.com
            0xC0, 0x0F, // Pointer to pointer to www.yahoo.com
            0x03, 0x77, 0x77, 0x77,
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65,
            0x03, 0x63, 0x6f, 0x6d,
            0x00,
        ];

        let mut buf = BytePacketBuffer::from_raw_data(packet);
        assert_eq!("www.yahoo.com", buf.read_qname());
        assert_eq!("www.yahoo.com", buf.read_qname());
        assert_eq!("www.yahoo.com", buf.read_qname());
        assert_eq!("www.google.com", buf.read_qname());
    }

    #[test]
    fn serialize_u8() {
        let ref mut serializer = BytePacketBuffer::new();

        let res = serializer.serialize_u8(0xDE);
        assert!(res.is_ok());

        let res = serializer.serialize_u8(0xAD);
        assert!(res.is_ok());

        assert_eq!(&[0xDE, 0xAD], &serializer.buf[..2]);
    }

    #[test]
    fn out_of_range_serialize_u8() {
        let ref mut serializer = BytePacketBuffer::new();
        serializer.pos = 512;

        let res = serializer.serialize_u8(0xDE);
        assert!(res.is_err());
    }

    #[test]
    fn serialize_u16() {
        let ref mut serializer = BytePacketBuffer::new();

        let res = serializer.serialize_u16(0xDEAD);
        assert!(res.is_ok());

        let res = serializer.serialize_u16(0xBEEF);
        assert!(res.is_ok());

        assert_eq!(&[0xDE, 0xAD, 0xBE, 0xEF], &serializer.buf[..4]);
    }

    #[test]
    fn out_of_range_serialize_u16() {
        let ref mut serializer = BytePacketBuffer::new();
        serializer.pos = 512;

        let res = serializer.serialize_u16(0xDEAD);
        assert!(res.is_err());
    }

    #[test]
    fn serialize_u32() {
        let ref mut serializer = BytePacketBuffer::new();

        let res = serializer.serialize_u32(0xDEAD_BEEF);
        assert!(res.is_ok());

        assert_eq!(&[0xDE, 0xAD, 0xBE, 0xEF], &serializer.buf[..4]);
    }

    #[test]
    fn out_of_range_serialize_u32() {
        let ref mut serializer = BytePacketBuffer::new();
        serializer.pos = 512;

        let res = serializer.serialize_u32(0xDEAD_BEEF);
        assert!(res.is_err());
    }

    #[test]
    fn serialize_qname() {
        let ref mut serializer = BytePacketBuffer::new();

        let res = serializer.serialize_qname("www.google.com");
        assert!(res.is_ok());

        let res = serializer.serialize_qname("www.yahoo.com");
        assert!(res.is_ok());

        assert_eq!(&[
            0x03, 0x77, 0x77, 0x77,
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65,
            0x03, 0x63, 0x6f, 0x6d,
            0x00,
            0x03, 0x77, 0x77, 0x77,
            0x05, 0x79, 0x61, 0x68, 0x6f, 0x6f,
            0x03, 0x63, 0x6f, 0x6d,
            0x00,
        ], &serializer.buf[..31]);
    }
}