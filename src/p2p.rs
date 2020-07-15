use crate::connection::Connection;
//use crate::error::Error as TorrentError;
use crate::message::Message;
use crate::torrent::TorrentFile;
use crate::tracker::{request_peers, Peer};
use rand::{self, Rng};
use std::error::Error;
use std::path::Path;

#[derive(Debug)]
struct MyError;

#[derive(Debug)]
pub struct Progress {
    index: u64,
    buf: Vec<u8>,
    downloaded: u64,
    requested: u64,
    backlog: u64,
}

impl Progress {
    fn new() -> Self {
        Progress {
            index: 0,
            buf: vec![],
            downloaded: 0,
            requested: 0,
            backlog: 0,
        }
    }
}

#[derive(Debug)]
pub struct Torrent {
    torrent_file: TorrentFile,
    peers: Vec<Peer>,
    progress: Progress,
    peer_id: Vec<u8>,
}

impl Torrent {
    pub fn new(path: &Path) -> Result<Self, Box<dyn Error>> {
        let torrent_file = TorrentFile::open(path)?;
        let peer_id = rand::thread_rng().gen::<[u8; 20]>().to_vec();
        let port = 6881; // FIXME: what does this mean?
        let tracker_response = request_peers(&torrent_file, &peer_id, &port)?;
        let peers = tracker_response.peers; // FIXME: keep tracker_response.interval?
        let progress = Progress::new();
        Ok(Self {
            torrent_file,
            peers,
            progress,
            peer_id,
        })
    }

    pub fn download(&mut self) -> Result<(), Box<dyn Error>> {
        let peer = self.peers.pop().ok_or("ERR: No more peers".to_string())?;
        // FIXME: clones are whack
        let mut conn = Connection::connect(
            peer.clone(),
            self.torrent_file.info_hash.clone(),
            self.peer_id.clone(),
        )?;

        while self.progress.index < self.torrent_file.length {
            // TODO: handle failures by switching to new peer
            // Perhaps new_peer method could help here and ^^
            let piece = self.download_piece(&mut conn);
        }

        self.save()

        // Connect to one peer

        // Loop: download piece(i) for all i
        // Optimization: If it fails for some piece, connect to a new peer
        // Optimization: Threads for every peers, eating from unbounded crossbeam queue, and
        // feeding results to diff crossbeam queue
    }

    // FIXME: can we infer index?
    fn download_piece(&mut self, conn: &mut Connection) -> Result<(), Box<dyn Error>> {
        // Make sure we have permission to download
        if conn.choked {
            println!("choked\n\n");
            conn.send_unchoke()?;
            conn.send_interested()?;
            self.receive_unchoke(conn)?;
        }

        // Download
        let block_size = 1000;
        conn.send_request(
            // FIXME: base conversions are whack
            self.progress.index as u32,
            self.progress.requested as u32,
            block_size,
        )?;
        self.receive_piece(conn)
    }

    fn receive_unchoke(&mut self, conn: &mut Connection) -> Result<(), Box<dyn Error>> {
        loop {
            if let Message::Unchoke = Message::read(&conn.stream)? {
                conn.choked = false;
                println!("Unchoked");
                return Ok(());
            }
        }
    }
    fn receive_piece(&mut self, conn: &mut Connection) -> Result<(), Box<dyn Error>> {
        loop {
            match Message::read(&conn.stream)? {
                Message::Choke => {
                    conn.choked = true;
                    return Ok(());
                }
                Message::Piece(index, requested, data) => {
                    println!("got piece: {} {} {}", index, requested, data.len());
                    return Ok(());
                }
                _ => println!("ignoring message"),
            }
        }
    }

    fn save(&self) -> Result<(), Box<dyn Error>> {
        // Once we're done downloading, save to a (use TorrentFile.name)
        Ok(())
    }
}
