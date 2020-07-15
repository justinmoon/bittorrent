use serde::{Deserialize, Serialize};
use serde_bencode;
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};
use std::convert::TryInto;
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
struct BencodeInfo {
    name: String,
    length: u64,
    #[serde(rename = "piece length")]
    piece_length: u64,
    pieces: ByteBuf,
}

#[derive(Debug, Deserialize)]
struct BencodeTorrent {
    announce: String,
    info: BencodeInfo,
}

#[derive(Debug, Deserialize)]
pub struct TorrentFile {
    name: String,
    pub announce: String,
    pub info_hash: Vec<u8>,
    pub length: u64,
    piece_length: u64,
    pub piece_hashes: Vec<[u8; 20]>,
}

impl BencodeInfo {
    fn hash(&self) -> Result<Vec<u8>, serde_bencode::Error> {
        let bytes = serde_bencode::to_bytes(self)?;
        let mut hasher = Sha1::new();

        hasher.input(bytes);

        Ok(hasher.result().to_vec())
    }
}

impl BencodeTorrent {
    fn to_torrent_file(self) -> Result<TorrentFile, serde_bencode::Error> {
        // Check valid number of pieces
        assert!(self.info.pieces.len() % 20 == 0);

        // Convert into hashes
        let mut piece_hashes: Vec<[u8; 20]> = vec![];
        for chunk in self.info.pieces.chunks(20) {
            piece_hashes.push(chunk.try_into().unwrap())
        }

        Ok(TorrentFile {
            info_hash: self.info.hash()?,
            name: self.info.name,
            announce: self.announce,
            length: self.info.length,
            piece_length: self.info.piece_length,
            piece_hashes,
        })
    }
}

impl TorrentFile {
    pub fn open(path: &Path) -> Result<TorrentFile, Box<dyn Error>> {
        let file = fs::read(path)?;
        let bencode_torrent = serde_bencode::from_bytes::<BencodeTorrent>(&file)?;
        let torrent = bencode_torrent.to_torrent_file()?;

        Ok(torrent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fs::File;

    #[test]
    pub fn test_it() {
        let ben_path = Path::new("data/archlinux-2019.12.01-x86_64.iso.torrent");
        let torrent = TorrentFile::open(&ben_path).unwrap();

        let json_path = Path::new("data/archlinux-2019.12.01-x86_64.iso.torrent.json");
        let json: Value = serde_json::from_reader(File::open(json_path).unwrap()).unwrap();

        assert_eq!(json["Announce"], torrent.announce);
        assert_eq!(json["PieceLength"], torrent.piece_length);
        assert_eq!(json["Length"], torrent.length);
        assert_eq!(json["Name"], torrent.name);
        assert_eq!(*json["InfoHash"].as_array().unwrap(), torrent.info_hash);

        for (i, hash) in torrent.piece_hashes.iter().enumerate() {
            for (j, byte) in hash.iter().enumerate() {
                assert_eq!(json["PieceHashes"][i][j], *byte);
            }
        }
    }
}
