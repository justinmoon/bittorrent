use crate::p2p::Torrent;
use std::error::Error;
use std::io;
use std::path::Path;

mod bitfield;
mod connection;
mod error;
mod message;
mod p2p;
mod torrent;
mod tracker;

fn main() {
    //let input = read_input().unwrap();
    //let path = Path::new(&input);
    let path = Path::new("data/ubuntu-18.04.4-desktop-amd64.iso.torrent");
    let mut torrent = Torrent::new(&path).unwrap();
    torrent.download().unwrap()
}

fn read_input() -> Result<String, Box<dyn Error>> {
    let mut input = String::new();

    println!("Path of the torrent");
    io::stdin().read_line(&mut input)?;

    input = input.trim().parse()?;

    Ok(input)
}
