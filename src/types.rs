use bytes::{Buf, Bytes};

#[derive(Debug, Default, Clone)]
pub struct DNSHeader {
    pub id: u16,
    pub qr: u8,
    pub opcode: u8,
    pub aa: u8,
    pub tc: u8,
    pub rd: u8,
    pub ra: u8,
    pub z: u8,
    pub ad: u8,
    pub cd: u8,
    pub rcode: u8,
    pub qdcount: u16,
    pub ancount: u16,
    pub nscount: u16,
    pub arcount: u16,
}

impl DNSHeader {
    pub fn from_bytes(buf: &mut impl Buf) -> anyhow::Result<Self> {
        let mut header = DNSHeader::default();
        header.id = buf.get_u16();

        let flags = buf.get_u16();
        header.qr = (flags >> 15) as u8;
        header.opcode = ((flags >> 11) & 0b1111) as u8;
        header.aa = ((flags >> 10) & 0b1) as u8;
        header.tc = ((flags >> 9) & 0b1) as u8;
        header.rd = ((flags >> 8) & 0b1) as u8;
        header.ra = ((flags >> 7) & 0b1) as u8;
        header.z = ((flags >> 6) & 0b1) as u8;
        header.ad = ((flags >> 5) & 0b1) as u8;
        header.cd = ((flags >> 4) & 0b1) as u8;
        header.rcode = (flags & 0b1111) as u8;

        header.qdcount = buf.get_u16();
        header.ancount = buf.get_u16();
        header.nscount = buf.get_u16();
        header.arcount = buf.get_u16();

        Ok(header)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.id.to_be_bytes());

        let mut flags = 0u16;
        flags |= (self.qr as u16) << 15;
        flags |= (self.opcode as u16) << 11;
        flags |= (self.aa as u16) << 10;
        flags |= (self.tc as u16) << 9;
        flags |= (self.rd as u16) << 8;
        flags |= (self.ra as u16) << 7;
        flags |= (self.z as u16) << 6;
        flags |= (self.ad as u16) << 5;
        flags |= (self.cd as u16) << 4;
        flags |= self.rcode as u16;
        buf.extend_from_slice(&flags.to_be_bytes());

        buf.extend_from_slice(&self.qdcount.to_be_bytes());
        buf.extend_from_slice(&self.ancount.to_be_bytes());
        buf.extend_from_slice(&self.nscount.to_be_bytes());
        buf.extend_from_slice(&self.arcount.to_be_bytes());

        buf
    }
}

#[derive(Debug, Default, Clone)]
pub struct DNSQuestion {
    pub qname: String,
    pub qtype: u16,
    pub qclass: u16,
}

impl DNSQuestion {
    pub fn from_bytes(buf: &mut impl Buf, buf_view: &[u8]) -> anyhow::Result<Self> {
        let mut question = DNSQuestion::default();
        question.qname = Self::read_qname(buf, buf_view)?;
        question.qtype = buf.get_u16();
        question.qclass = buf.get_u16();
        Ok(question)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&Self::write_qname(&self.qname));
        buf.extend_from_slice(&self.qtype.to_be_bytes());
        buf.extend_from_slice(&self.qclass.to_be_bytes());
        buf
    }

    fn read_qname(buf: &mut impl Buf, buf_view: &[u8]) -> anyhow::Result<String> {
        let mut qname = String::new();
        loop {
            let len = buf.get_u8();

            if len == 0 {
                break;
            }

            if len & 0b1100_0000 == 0b1100_0000 {
                let offset = ((len & 0b0011_1111) as u16) << 8 | buf.get_u8() as u16;
                let mut buf = &buf_view[offset as usize..];

                let label = Self::read_qname(&mut buf, buf_view)?;
                qname.push_str(&label);
                qname.push('.');

                break;
            } else {
                let buffer = buf.copy_to_bytes(len as usize);
                qname.push_str(&String::from_utf8_lossy(&buffer));
                qname.push('.');
            }
        }
        Ok(qname)
    }

    fn write_qname(qname: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        for label in qname.split('.') {
            if label.is_empty() {
                break;
            }

            buf.push(label.len() as u8);
            buf.extend_from_slice(label.as_bytes());
        }
        buf.push(0);
        buf
    }
}

#[derive(Debug, Default, Clone)]
pub struct DNSAnswer {
    pub name: String,
    pub rtype: u16,
    pub rclass: u16,
    pub ttl: u32,
    pub rdlength: u16,
    pub rdata: Vec<u8>,
}

impl DNSAnswer {
    pub fn from_bytes(buf: &mut impl Buf, global_buf: &[u8]) -> anyhow::Result<Self> {
        let mut answer = DNSAnswer {
            name: DNSQuestion::read_qname(buf, global_buf)?,
            rtype: buf.get_u16(),
            rclass: buf.get_u16(),
            ttl: buf.get_u32(),
            rdlength: buf.get_u16(),
            rdata: Vec::new(),
        };
        
        answer.rdata = buf.copy_to_bytes(answer.rdlength as usize).to_vec();

        if answer.rdlength != answer.rdata.len() as u16 {
            println!("Invalid answer: {:?}", answer);

            anyhow::bail!("Invalid answer");
        }

        Ok(answer)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&DNSQuestion::write_qname(&self.name));
        buf.extend_from_slice(&self.rtype.to_be_bytes());
        buf.extend_from_slice(&self.rclass.to_be_bytes());
        buf.extend_from_slice(&self.ttl.to_be_bytes());
        buf.extend_from_slice(&self.rdlength.to_be_bytes());
        buf.extend_from_slice(&self.rdata);
        buf
    }
}