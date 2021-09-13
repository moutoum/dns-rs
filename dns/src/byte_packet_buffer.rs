const DEFAULT_BUFFER_SIZE: usize = 512;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

pub struct BytePacketBuffer {
    buf: [u8; DEFAULT_BUFFER_SIZE],
    pos: usize,
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

    fn pos(&self) -> usize {
        self.pos
    }

    fn step(&mut self, steps: usize) {
        self.pos += steps
    }

    fn seek(&mut self, pos: usize) {
        self.pos = pos
    }

    pub fn read(&mut self) -> Result<u8> {
        if self.pos >= DEFAULT_BUFFER_SIZE {
            return Err("out of range".into());
        }

        let c = self.buf[self.pos];
        self.pos += 1;
        Ok(c)
    }

    fn get(&self, pos: usize) -> Result<u8> {
        if pos >= DEFAULT_BUFFER_SIZE {
            return Err("out of range".into());
        }

        Ok(self.buf[pos])
    }

    fn get_range(&self, pos: usize, len: usize) -> Result<&[u8]> {
        if pos + len >= DEFAULT_BUFFER_SIZE {
            return Err("out of range".into());
        }

        Ok(&self.buf[pos..pos + len])
    }

    pub fn read_n(&mut self, len: usize) -> Result<Vec<u8>> {
        if self.pos + len >= DEFAULT_BUFFER_SIZE {
            return Err("out of range".into());
        }

        let out = Vec::from(&self.buf[self.pos..self.pos + len]);
        self.step(len);
        Ok(out)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let first_byte = (self.read()? as u16) << 8;
        let second_byte = self.read()? as u16;
        Ok(first_byte | second_byte)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let first_byte = (self.read()? as u32) << 24;
        let second_byte = (self.read()? as u32) << 16;
        let third_byte = (self.read()? as u32) << 8;
        let fourth_byte = self.read()? as u32;
        Ok(first_byte | second_byte | third_byte | fourth_byte)
    }

    pub fn read_qname(&mut self) -> Result<String> {
        let mut out = String::new();
        let mut delimiter = "";
        let mut pos = self.pos();
        let mut jumped = false;

        loop {
            let len = self.get(pos)?;
            pos += 1;

            match len {
                // End of qname.
                0 => break,

                // Pointer to a qname in the packet.
                _ if len & 0xC0 == 0xC0 => {
                    self.seek(pos + 1);
                    let b1 = len as u16 ^ 0xC0;
                    let b2 = self.get(pos)? as u16;
                    let offset = (b1 << 8) | b2;
                    pos = offset as usize;
                    jumped = true;
                }

                // Normal case where the first byte is the length of the following label.
                _ => {
                    let label = self.get_range(pos, len as usize)?;
                    out.push_str(delimiter);
                    out.push_str(&String::from_utf8_lossy(label).to_lowercase());
                    delimiter = ".";
                    pos += len as usize;
                }
            }
        }

        if !jumped {
            self.seek(pos);
        }

        Ok(out)
    }
}

mod test {
    use crate::byte_packet_buffer::BytePacketBuffer;

    #[test]
    fn read_u8() {
        let mut buf = BytePacketBuffer::new();
        buf.buf[0] = 0xDE;
        buf.buf[1] = 0xAD;
        assert_eq!(0xDE, buf.read().unwrap());
        assert_eq!(0xAD, buf.read().unwrap());

        buf.pos = 1024;
        assert!(buf.read().is_err())
    }

    #[test]
    fn read_u16() {
        let mut buf = BytePacketBuffer::new();
        buf.buf[0] = 0xDE;
        buf.buf[1] = 0xAD;
        assert_eq!(0xDEAD, buf.read_u16().unwrap());

        buf.pos = 1024;
        assert!(buf.read_u16().is_err())
    }

    #[test]
    fn read_u32() {
        let mut buf = BytePacketBuffer::new();
        buf.buf[0] = 0xDE;
        buf.buf[1] = 0xAD;
        buf.buf[2] = 0xBE;
        buf.buf[3] = 0xEF;
        assert_eq!(0xDEAD_BEEF, buf.read_u32().unwrap());

        buf.pos = 1024;
        assert!(buf.read_u32().is_err())
    }

    #[test]
    fn read_qname() {
        let packet: &[u8] = &[
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, // len=6 label="google"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
        ];

        let mut buf = BytePacketBuffer::new();
        buf.buf[..packet.len()].copy_from_slice(packet);

        assert_eq!("www.google.com", buf.read_qname().unwrap());
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

        let mut buf = BytePacketBuffer::new();
        buf.buf[..packet.len()].copy_from_slice(packet);

        assert_eq!("www.google.com", buf.read_qname().unwrap());
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

        let mut buf = BytePacketBuffer::new();
        buf.buf[..packet.len()].copy_from_slice(packet);

        assert_eq!("www.google.com", buf.read_qname().unwrap());
        assert_eq!("www.yahoo.com", buf.read_qname().unwrap());
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

        let mut buf = BytePacketBuffer::new();
        buf.buf[..packet.len()].copy_from_slice(packet);

        assert_eq!("www.google.com", buf.read_qname().unwrap());
        assert_eq!("www.yahoo.com", buf.read_qname().unwrap());
        assert_eq!("www.yahoo.com", buf.read_qname().unwrap());
        assert_eq!("www.google.com", buf.read_qname().unwrap());
    }

    #[test]
    fn read_qname_pointer_to_pointer() {
        let packet: &[u8] = &[
            0x03, 0x77, 0x77, 0x77, // len=3 label="www"
            0x05, 0x79, 0x61, 0x68, 0x6f, 0x6f, // len=6 label="yahoo"
            0x03, 0x63, 0x6f, 0x6d, // len=3 label="com"
            0x00,
            0xC0, 0x00, // Pointer to www.yahoo.com
            0xC0, 0x0F // Pointer to pointer to www.yahoo.com
        ];

        let mut buf = BytePacketBuffer::new();
        buf.buf[..packet.len()].copy_from_slice(packet);

        assert_eq!("www.yahoo.com", buf.read_qname().unwrap());
        assert_eq!("www.yahoo.com", buf.read_qname().unwrap());
        assert_eq!("www.yahoo.com", buf.read_qname().unwrap());
    }
}