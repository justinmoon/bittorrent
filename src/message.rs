// MessageType enum
// Message does serialization, perhaps reads from socket?

use byteorder::{BigEndian, ByteOrder};
use std::io::{self, Read};
use std::net::TcpStream;

#[derive(Debug)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request(u32, u32, u32),
    Piece(u32, u32, Vec<u8>),
    Cancel,
}

impl Message {
    // FIXME: this is a really dumb new() method
    pub fn new(id: u8, payload: &[u8]) -> Message {
        println!("Message::new({} {})", id, payload.len());
        let msg = match id {
            0 => Message::Choke,
            1 => Message::Unchoke,
            2 => Message::Interested,
            3 => Message::NotInterested,
            4 => Message::Have(BigEndian::read_u32(payload)),
            5 => Message::Bitfield(payload.to_vec()),
            6 => {
                let index = BigEndian::read_u32(&payload[..4]);
                let begin = BigEndian::read_u32(&payload[4..8]);
                let length = BigEndian::read_u32(&payload[8..]);

                Message::Request(index, begin, length)
            }
            7 => {
                println!("len: {:?}", &payload.len());
                println!("index: {:?}", &payload[0..4]);
                let index = BigEndian::read_u32(&payload[..4]);
                println!("begin: {:?}", &payload[4..8]);
                let begin = BigEndian::read_u32(&payload[4..8]);
                let piece = payload[8..].to_vec();

                Message::Piece(index, begin, piece)
            }
            8 => Message::Cancel,
            _ => panic!("Bad message ID: {}", id),
        };
        println!("msg: {:?}", msg);
        msg
    }

    pub fn read(mut conn: &TcpStream) -> Result<Message, io::Error> {
        let mut msg_len = [0; 4];

        conn.read_exact(&mut msg_len)?;

        let msg_len = BigEndian::read_u32(&msg_len);
        println!("msg len: {:?}", msg_len);
        let mut msg = Vec::new();

        conn.take(msg_len as u64).read_to_end(&mut msg)?;

        if msg_len > 0 {
            Ok(Message::new(msg[0], &msg[1..]))
        } else {
            Ok(Message::KeepAlive)
        }
    }

    pub fn serialize(&self, payload: &[u8]) -> Vec<u8> {
        let length = payload.len() + 1;
        let id = match self {
            Message::Choke => 0,
            Message::Unchoke => 1,
            Message::Interested => 2,
            Message::NotInterested => 3,
            Message::Request(_, _, _) => 6,
            _ => 4,
        };

        let mut buf = [0; 5];
        BigEndian::write_u32(&mut buf, length as u32);
        buf[4] = id;
        let mut done = Vec::from(buf);
        done.extend(payload);
        println!("serializing: {:?}", done);
        done
    }
}

#[cfg(test)]
mod tests {}
