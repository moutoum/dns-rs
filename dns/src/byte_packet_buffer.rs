const DEFAULT_BUFFER_SIZE: usize = 512;

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

    fn get_u8(&self, pos: usize) -> u8 {
        assert!(pos < DEFAULT_BUFFER_SIZE, "pos out of range: {:?} >= {:?}", pos, DEFAULT_BUFFER_SIZE);
        self.buf[pos]
    }

    fn get_range(&self, pos: usize, len: usize) -> &[u8] {
        assert!(pos + len < DEFAULT_BUFFER_SIZE, "pos out of range: {:?} >= {:?}", pos + len, DEFAULT_BUFFER_SIZE);
        &self.buf[pos..pos + len]
    }

    pub fn read_u8(&mut self) -> u8 {
        assert!(self.pos < DEFAULT_BUFFER_SIZE, "pos out of range: {:?} >= {:?}", self.pos, DEFAULT_BUFFER_SIZE);
        let c = self.buf[self.pos];
        self.pos += 1;
        c
    }

    pub fn read_n(&mut self, len: usize) -> Vec<u8> {
        assert!(self.pos + len < DEFAULT_BUFFER_SIZE, "pos out of range: {:?} >= {:?}", self.pos + len, DEFAULT_BUFFER_SIZE);
        let out = self.get_range(self.pos, len).into();
        self.step(len);
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
        let mut pos = self.pos();
        let mut jumped = false;

        loop {
            let len = self.get_u8(pos);
            pos += 1;

            match len {
                // End of qname.
                0 => break,

                // Pointer to a qname in the packet.
                _ if len & 0xC0 == 0xC0 => {
                    self.seek(pos + 1);
                    let b1 = len as u16 ^ 0xC0;
                    let b2 = self.get_u8(pos) as u16;
                    let offset = (b1 << 8) | b2;
                    pos = offset as usize;
                    jumped = true;
                }

                // Normal case where the first byte is the length of the following label.
                _ => {
                    let label = self.get_range(pos, len as usize);
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

        out
    }
}

mod test {
    use crate::byte_packet_buffer::BytePacketBuffer;

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
            0xC0, 0x0F // Pointer to pointer to www.yahoo.com
        ];

        let mut buf = BytePacketBuffer::from_raw_data(packet);
        assert_eq!("www.yahoo.com", buf.read_qname());
        assert_eq!("www.yahoo.com", buf.read_qname());
        assert_eq!("www.yahoo.com", buf.read_qname());
    }
}