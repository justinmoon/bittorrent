use crate::message::Message;
use crate::{torrent::TorrentFile, tracker::Peer};
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
    stream: TcpStream,
}

impl Handshake {
    fn new(stream: TcpStream, info_hash: Vec<u8>, peer_id: Vec<u8>) -> Handshake {
        Handshake {
            pstr: String::from("BitTorrent protocol"),
            info_hash,
            peer_id,
            stream,
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.push(self.pstr.len() as u8);
        result.extend(self.pstr.as_bytes());
        result.extend(&[0; 8]);
        result.extend(&self.info_hash);
        result.extend(&self.peer_id);

        result
    }

    fn check_response(&self, b: &[u8]) -> Result<(), Box<dyn Error>> {
        // Deserialize (mainly care about info hash)
        let pstr_len = 19;
        let pstr = String::from_utf8(b[1..pstr_len + 1].to_vec())?;
        let info_hash = b[pstr_len + 1 + 8..pstr_len + 1 + 8 + 20].to_vec();
        let peer_id = &b[pstr_len + 1 + 8 + 20..];
        let peer_id = peer_id.to_vec();

        if self.info_hash.eq(&info_hash) {
            println!("Successful handshake.");

            Ok(())
        } else {
            println!(
                "Expected info_hash: {:?} but got {:?}",
                self.info_hash, info_hash
            );
            Err(Box::try_from("Incorrect info_hash.").unwrap())
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // Initiate handshake
        self.stream.write_all(&self.serialize().as_slice())?;

        // Receive and verify response
        let mut buf = [0; 68];
        self.stream.read_exact(&mut buf)?;
        self.check_response(&buf)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Connection {
    pub stream: TcpStream,
    pub choked: bool,
    pub peer: Peer,
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>,
    pub bitfield: Vec<u8>,
}

impl Connection {
    // TODO: this should execute the handshake
    pub fn connect(
        peer: Peer,
        info_hash: Vec<u8>,
        peer_id: Vec<u8>,
    ) -> Result<Connection, Box<dyn Error>> {
        // Create TCP stream
        let addr = SocketAddr::new(IpAddr::from(peer.ip), peer.port);
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;

        // Execute bittorrent handshake with peer
        // FIXME: cloning here is lame
        Handshake::new(stream.try_clone()?, info_hash.clone(), peer_id.clone()).run()?;

        // Receive bitfield
        let msg = Message::read(&stream).unwrap();
        let bitfield = match msg {
            Message::Bitfield(bitfield) => bitfield,
            _ => panic!("BAD BAD BAD"),
        };

        // Instantiate connection
        Ok(Connection {
            stream,
            choked: true,
            peer,
            info_hash,
            peer_id,
            bitfield,
        })
    }

    pub fn send_unchoke(&mut self) -> Result<(), Box<dyn Error>> {
        let msg = Message::Unchoke;
        let payload = vec![];
        let bytes = msg.serialize(&payload);
        self.stream.write_all(&bytes)?;
        Ok(())
    }

    pub fn send_interested(&mut self) -> Result<(), Box<dyn Error>> {
        let msg = Message::Interested;
        let payload = vec![];
        let bytes = msg.serialize(&payload);
        self.stream.write_all(&bytes)?;
        Ok(())
    }

    pub fn send_request(
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
    use crate::torrent::TorrentFile;
    use crate::tracker::request_peers;
    use env_logger;
    use rand::Rng;
    use std::path::Path;

    #[test]
    pub fn test_connection() {
        env_logger::init();
        // Get some peers
        let ben_path = Path::new("data/ubuntu-18.04.4-desktop-amd64.iso.torrent");
        let torrent = TorrentFile::open(&ben_path).unwrap();
        let peer_id = rand::thread_rng().gen::<[u8; 20]>().to_vec();
        let port = 6881;
        let peers_response = request_peers(&torrent, &peer_id, &port).unwrap();

        // connect to the first peer
        let peer = peers_response.peers[2].clone();
        let mut conn = Connection::connect(peer, torrent.info_hash, peer_id).unwrap();

        // download chunks
        conn.download().unwrap();
    }
}
