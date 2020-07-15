use crate::torrent::TorrentFile;
use byteorder::{BigEndian, ByteOrder};
use core::fmt;
use percent_encoding::percent_encode_byte;
use reqwest;
use reqwest::Url;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};
use std::error::Error;
use std::net::Ipv4Addr;
use std::time::Duration;

struct PeerVecVisitor;
#[derive(Debug, Deserialize, Clone)]
pub struct Peer {
    pub ip: Ipv4Addr,
    pub port: u16,
}
#[derive(Debug, Deserialize)]
pub struct TrackerResponse {
    interval: u32,
    #[serde(deserialize_with = "Peer::vec_from_bytes")]
    pub peers: Vec<Peer>,
}

impl Peer {
    fn from_bytes(b: &[u8]) -> Peer {
        let ip = Ipv4Addr::new(b[0], b[1], b[2], b[3]);
        //        let port = (b[4] as u16) * 256 + (b[5] as u16);
        let port = BigEndian::read_u16(&[b[4], b[5]]);

        Peer { ip, port }
    }

    fn vec_from_bytes<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Peer>, D::Error> {
        d.deserialize_byte_buf(PeerVecVisitor)
    }
}

impl<'de> Visitor<'de> for PeerVecVisitor {
    type Value = Vec<Peer>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("byte array")
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        Ok(v.chunks(6).map(Peer::from_bytes).collect())
    }
}

pub fn request_peers(
    torrent: &TorrentFile,
    peer_id: &Vec<u8>,
    port: &u16,
) -> Result<TrackerResponse, Box<dyn Error>> {
    let url_hash = (&torrent.info_hash)
        .into_iter()
        .map(|b| percent_encode_byte(*b))
        .collect::<String>();
    let peer_id_es = peer_id
        .into_iter()
        .map(|b| percent_encode_byte(*b))
        .collect::<String>();
    let base_url = format!(
        "{}?info_hash={}&peer_id={}",
        torrent.announce, url_hash, peer_id_es
    );
    let url = Url::parse_with_params(
        base_url.as_str(),
        &[
            ("port", port.to_string()),
            ("uploaded", "0".to_string()),
            ("downloaded", "0".to_string()),
            ("compact", "1".to_string()),
            ("left", torrent.length.to_string()),
        ],
    )?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let mut res = client.get(url).send()?;
    let mut buf = Vec::new();

    // FIXME: this should check the status code

    res.copy_to(&mut buf)?;

    let tracker_response = serde_bencode::from_bytes::<TrackerResponse>(&buf.as_slice())?;

    Ok(tracker_response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use std::path::Path;

    #[test]
    pub fn test_request_peers() {
        // Request some peers
        let ben_path = Path::new("data/ubuntu-18.04.4-desktop-amd64.iso.torrent");
        let torrent = TorrentFile::open(&ben_path).unwrap();
        let peer_id = rand::thread_rng().gen::<[u8; 20]>().to_vec();
        let port = 6881;
        let peers_response = request_peers(&torrent, &peer_id, &port).unwrap();

        // Check that we got some peers
        assert!(peers_response.peers.len() > 0);
    }
}
