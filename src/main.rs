use crate::connection::Connection;
use crate::torrent::Torrent;
use rand::Rng;
use std::error::Error;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;

mod bitfield;
mod connection;
mod message;
mod torrent;
mod tracker;

fn main() {
    let input = read_input().unwrap();
    let path = Path::new(&input);
    let torrent = Torrent::open(path).unwrap();
    let peer_id = rand::thread_rng().gen::<[u8; 20]>().to_vec();
    let port = 6881;
    let mut res = tracker::request_peers(&torrent, &peer_id, &port).unwrap();

    let test_peer = res.peers.pop().unwrap();

    let peer_addr = SocketAddr::new(
        IpAddr::from(test_peer.ip.to_owned()),
        test_peer.port.to_owned(),
    );
    let conn = Connection::connect(test_peer, torrent.info_hash, peer_id);

    match conn {
        Ok(mut conn) => {
            let hs = conn.complete_handshake().unwrap();
            let msg = message::Message::read(&conn.stream).unwrap();

            println!("{:?}", msg);
        }
        Err(_) => println!("Connection with {} timed out.", peer_addr),
    }

    //    let hs = conn.complete_handshake().unwrap();
    //    let msg = message::Message::read(&conn.stream).unwrap();
    //
    //    println!("{:?}", msg);
}

fn read_input() -> Result<String, Box<dyn Error>> {
    let mut input = String::new();

    println!("Path of the torrent");
    io::stdin().read_line(&mut input)?;

    input = input.trim().parse()?;

    Ok(input)
}
