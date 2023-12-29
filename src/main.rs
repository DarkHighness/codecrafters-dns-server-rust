mod resolver;
mod types;

use std::net::UdpSocket;

use bytes::BytesMut;
use resolver::Resolver;

use crate::types::{DNSAnswer, DNSHeader, DNSQuestion};

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let upstream = args
        .get(2)
        .cloned()
        .unwrap_or("8.8.8.8:53".to_string())
        .parse()?;

    let resolver = Resolver::new(upstream);

    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; 512];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                println!("Received {} bytes from {}", size, source);

                let buf_view = &buf[..size];
                let mut buf = BytesMut::from(&buf_view[..]);

                let request_header =
                    DNSHeader::from_bytes(&mut buf).expect("Failed to parse header");

                println!("Request header: {:?}", request_header);

                let question_cnt = request_header.qdcount;

                println!("Question count: {}", question_cnt);

                let request_questions = (0..question_cnt)
                    .map(|_| {
                        DNSQuestion::from_bytes(&mut buf, &buf_view)
                            .expect("Failed to parse question")
                    })
                    .collect::<Vec<_>>();

                let mut response_header: DNSHeader = Default::default();
                response_header.id = request_header.id;
                response_header.qr = 1;
                response_header.opcode = request_header.opcode;
                response_header.qdcount = request_header.qdcount;
                response_header.rd = request_header.rd;
                response_header.ancount = request_header.qdcount;

                if response_header.opcode != 0 {
                    response_header.rcode = 4;
                }

                let response_answers = request_questions
                    .iter()
                    .flat_map(|q| {
                        resolver.resolve(&request_header, q).expect("Failed to resolve")
                    })
                    .collect::<Vec<_>>();

                let response = [
                    response_header.to_bytes().as_slice(),
                    request_questions
                        .iter()
                        .map(|q| q.to_bytes())
                        .collect::<Vec<_>>()
                        .concat()
                        .as_slice(),
                    response_answers
                        .iter()
                        .map(|a| a.to_bytes())
                        .collect::<Vec<_>>()
                        .concat()
                        .as_slice(),
                ]
                .concat();

                println!("Sending {} bytes to {}", response.len(), source);
                println!("Bytes: {:?}", response);

                udp_socket
                    .send_to(&response, source)
                    .expect("Failed to send response");
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }

    Ok(())
}
