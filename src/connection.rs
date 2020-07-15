use crate::message::Message;
use crate::{torrent::Torrent, tracker::Peer};
use byteorder::{BigEndian, WriteBytesExt};
use std::convert::TryFrom;
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::string::FromUtf8Error;
use std::time::Duration;

#[derive(Debug)]
pub struct Handshake {
    pstr: String,
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
}

pub struct Connection {
    pub stream: TcpStream,
    chocked: bool,
    peer: Peer,
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
}

impl Handshake {
    fn new(info_hash: Vec<u8>, peer_id: Vec<u8>) -> Handshake {
        Handshake {
            pstr: String::from("BitTorrent protocol"),
            info_hash,
            peer_id,
        }
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.push(self.pstr.len() as u8);
        result.extend(self.pstr.as_bytes());
        result.extend(&[0; 8]);
        result.extend(&self.info_hash);
        result.extend(&self.peer_id);

        result
    }

    fn from_bytes(b: &[u8]) -> Result<Handshake, FromUtf8Error> {
        let pstr_len = 19;
        let pstr = String::from_utf8(b[1..pstr_len + 1].to_vec())?;
        let info_hash = b[pstr_len + 1 + 8..pstr_len + 1 + 8 + 20].to_vec();
        let peer_id = &b[pstr_len + 1 + 8 + 20..];
        let peer_id = peer_id.to_vec();

        Ok(Handshake {
            pstr,
            info_hash,
            peer_id,
        })
    }
}

impl Connection {
    pub fn connect(
        peer: Peer,
        info_hash: Vec<u8>,
        peer_id: Vec<u8>,
    ) -> Result<Connection, io::Error> {
        let addr = SocketAddr::new(IpAddr::from(peer.ip), peer.port);
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;

        //    conn.set_write_timeout(Some(Duration::from_secs(5)))?;
        //    conn.set_read_timeout(Some(Duration::from_secs(5)))?;

        Ok(Connection {
            stream,
            chocked: true,
            peer,
            info_hash,
            peer_id,
        })
    }

    pub fn complete_handshake(&mut self) -> Result<Handshake, Box<dyn Error>> {
        let hs = self.send_handshake()?;
        let res_hs = self.receive_handshake()?;

        if hs.info_hash.eq(&res_hs.info_hash) {
            println!("Successful handshake.");

            Ok(res_hs)
        } else {
            println!(
                "Expected info_hash: {:?} but got {:?}",
                hs.info_hash, res_hs.info_hash
            );

            Err(Box::try_from("Incorrect info_hash.").unwrap())
        }
    }

    fn send_handshake(&mut self) -> Result<Handshake, io::Error> {
        let hs = Handshake::new(self.info_hash.to_owned(), self.peer_id.to_owned());

        self.stream.write_all(&hs.as_bytes().as_slice())?;

        Ok(hs)
    }

    fn receive_handshake(&mut self) -> Result<Handshake, Box<dyn Error>> {
        let mut buf = [0; 68];

        self.stream.read_exact(&mut buf)?;

        let res_hs = Handshake::from_bytes(&buf)?;

        Ok(res_hs)
    }

    fn send_unchoke(&mut self) -> Result<(), Box<dyn Error>> {
        let msg = Message::Unchoke;
        let payload = vec![];
        let bytes = msg.serialize(&payload);
        self.stream.write_all(&bytes)?;
        Ok(())
    }

    fn send_interested(&mut self) -> Result<(), Box<dyn Error>> {
        let msg = Message::Interested;
        let payload = vec![];
        let bytes = msg.serialize(&payload);
        self.stream.write_all(&bytes)?;
        Ok(())
    }

    fn send_request(
        &mut self,
        index: u32,
        requested: u32,
        block_size: u32,
    ) -> Result<(), Box<dyn Error>> {
        let msg = Message::Request(index, requested, block_size);
        let mut payload = vec![];
        // WriteBytesExt
        payload.write_u32::<BigEndian>(index)?;
        payload.write_u32::<BigEndian>(requested)?;
        payload.write_u32::<BigEndian>(block_size)?;
        let bytes = msg.serialize(&payload);
        self.stream.write_all(&bytes)?;
        Ok(())
    }

    pub fn download(&mut self) -> Result<(), Box<dyn Error>> {
        // Tell peer we're ready
        self.send_unchoke()?;
        self.send_interested()?;

        // Wait for unchoke
        loop {
            let msg = Message::read(&self.stream)?;
            if let Message::Unchoke = msg {
                println!("Unchoked");
                break;
            }
        }

        // Track download progress
        let mut index = 0;
        let mut requested = 0;
        let block_size = 1000; // the max

        // Download pieces
        loop {
            // Request piece
            self.send_request(index, requested, block_size)?;
            println!("send request #{}", index);

            // Receive next piece
            loop {
                let msg = Message::read(&self.stream)?;
                println!("new msg: {:?}", msg);

                if let Message::Piece(_, _, _) = msg {
                    // FIXME: increment requested
                    index += 1;
                    break;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use crate::torrent::Torrent;
    use crate::tracker::request_peers;
    use env_logger;
    use rand::Rng;
    use std::path::Path;

    #[test]
    pub fn test_connection() {
        env_logger::init();
        // Get some peers
        let ben_path = Path::new("data/ubuntu-18.04.4-desktop-amd64.iso.torrent");
        let torrent = Torrent::open(&ben_path).unwrap();
        let peer_id = rand::thread_rng().gen::<[u8; 20]>().to_vec();
        let port = 6881;
        let peers_response = request_peers(&torrent, &peer_id, &port).unwrap();

        // connect to the first peer
        let peer = peers_response.peers[2].clone();
        let mut conn_result = Connection::connect(peer, torrent.info_hash, peer_id);

        // check we could complete connection and first message contained bitfield
        let mut conn = match conn_result {
            Ok(mut conn) => {
                conn.complete_handshake().unwrap();
                let msg = Message::read(&conn.stream).unwrap();
                match msg {
                    Message::Bitfield(_) => { /* what we expected */ }
                    _ => panic!("BAD BAD BAD"),
                };
                conn
            }
            Err(_) => panic!("BAD BAD WIRSE"),
        };

        // download chunks
        conn.download().unwrap();
    }
}
