//! **The FS server's own cost of a directory walk**, on this host's core (milestone 606 (a
//! directory walk costs what it does on Linux), provisional number).
//!
//! `walk_model` replays the requests `std` sends for `walk_pricing::walk` straight at the server
//! core, over an in-memory image behind the same cache the EL0 binary runs. No IPC, no caretaker,
//! no device: what is left is the server's work, which on an M-series host is the same core the
//! HVF guest runs on. Subtract it from the on-target walk and the remainder is the path.
//!
//! ```text
//! cargo run --release --manifest-path redoxfs_server/Cargo.toml --example walk_replay
//! ```

use std::time::Instant;

use filesystem_protocol::dir;
use filesystem_protocol::fixture::walk as tree;
use redoxfs::{DiskMemory, FileSystem};
use redoxfs_server::walk_model::{self, Counting};
use redoxfs_server::{CachedDisk, Server};

fn main() {
    let runs: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(51);
    let disk = DiskMemory::new(32 * 1024 * 1024);
    let fs = FileSystem::create(disk, None, 0, 0).expect("create");
    let mut srv = Server::open(fs.disk).expect("open");
    walk_model::stage(&mut srv, filesystem_protocol::fs::ROOT as u32).expect("stage");
    let disk = srv.into_disk();
    let mut srv = Server::open(CachedDisk::new(
        Counting::new(disk),
        redoxfs_server::CACHE_SLOTS,
    ))
    .expect("reopen");
    let root = srv
        .open_dir(0, tree::ROOT, dir::ENUMERATE | dir::READ | dir::DESCEND)
        .expect("open the tree");

    let reads0 = srv.disk_mut().inner().reads;
    let cold = walk_model::walk(&mut srv, root).expect("cold walk");
    let reads1 = srv.disk_mut().inner().reads;
    let mut ns = Vec::with_capacity(runs);
    for _ in 0..runs {
        let t0 = Instant::now();
        walk_model::walk(&mut srv, root).expect("warm walk");
        ns.push(t0.elapsed().as_nanos() as u64);
    }
    let reads2 = srv.disk_mut().inner().reads;
    ns.sort_unstable();
    // A namespace change forgets the memo; what is left is the block cache under it.
    let h = srv.create_file_at(0, "scratch").expect("create");
    srv.close(h).expect("close");
    srv.unlink(0, "scratch").expect("unlink");
    let reads3 = srv.disk_mut().inner().reads;
    let t0 = Instant::now();
    walk_model::walk(&mut srv, root).expect("walk after a change");
    let after_change = t0.elapsed().as_nanos();
    let reads4 = srv.disk_mut().inner().reads;
    let r = cold.requests;
    println!(
        "requests per walk {} (opendir {}, readdir {}, open {}, fstat {}, read {}, close {})",
        r.total(),
        r.opendir,
        r.readdir,
        r.open,
        r.fstat,
        r.read,
        r.close
    );
    println!(
        "device reads: cold walk {}, warm walk {:.1}",
        reads1 - reads0,
        (reads2 - reads1) as f64 / runs as f64
    );
    println!(
        "after a namespace change: device reads {}, server time {after_change} ns",
        reads4 - reads3
    );
    println!(
        "server time per warm walk: median {} ns, min {} ns, max {} ns ({runs} runs)",
        ns[runs / 2],
        ns[0],
        ns[runs - 1]
    );
}
