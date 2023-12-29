use std::net::{SocketAddr, UdpSocket};

use bytes::BytesMut;

use crate::types::{DNSAnswer, DNSHeader, DNSQuestion};

pub struct Resolver {
    upstream: SocketAddr,
    socket: UdpSocket,
}

impl Resolver {
    pub fn new(upstream: SocketAddr) -> Self {
        let socket = UdpSocket::bind("0.0.0.0:50035").expect("Failed to bind to address");
        Resolver { upstream, socket }
    }

    pub fn resolve(
        &self,
        header: &DNSHeader,
        question: &DNSQuestion,
    ) -> anyhow::Result<Vec<DNSAnswer>> {
        println!("Resolving {:?}", question);

        let buf = vec![header.to_bytes(), question.to_bytes()].concat();

        self.socket.send_to(&buf, self.upstream)?;

        let mut buf = [0; 512];
        let (size, _) = self.socket.recv_from(&mut buf)?;

        println!("Received {} bytes from upstream", size);

        let buf_view = &buf[..size];
        let mut buf = BytesMut::from(&buf_view[..]);

        let response_header = DNSHeader::from_bytes(&mut buf)?;
        let question_cnt = response_header.qdcount;
        let answer_cnt = response_header.ancount;

        println!("Response header from upstream: {:?}", response_header);
        assert_eq!(response_header.qr, 1);

        let response_questions = (0..question_cnt)
            .map(|_| DNSQuestion::from_bytes(&mut buf, &buf_view))
            .collect::<anyhow::Result<Vec<_>>>()?;

        println!("Response questions from upstream: {:?}", response_questions);

        let response_answers = (0..answer_cnt)
            .map(|_| DNSAnswer::from_bytes(&mut buf, &buf_view))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let response_answers = if response_answers.is_empty() {
            let mut fake_answer: DNSAnswer = Default::default();

            fake_answer.name = question.qname.clone();
            fake_answer.rtype = 1;
            fake_answer.rclass = 1;
            fake_answer.ttl = 60;
            fake_answer.rdlength = 4;
            fake_answer.rdata = vec![127, 0, 0, 1];

            vec![fake_answer]
        } else {
            response_answers
        };

        println!("Response answers from upstream: {:?}", response_answers);

        Ok(response_answers)
    }
}
