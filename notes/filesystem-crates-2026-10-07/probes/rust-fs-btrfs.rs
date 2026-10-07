// btrfs reader with a partial write path. Reads the named paths; with WRITE set, tries the
// copy-on-write overwrite (`Filesystem::write`) on the first path and reads it back. The device
// With CHECK set, also runs the crate's read-only checker (a subset of `btrfs check`). The device
// is a plain `File` behind a mutex rather than fs_core's `FileDevice`, which is unix/windows-only
// and would measure the host adapter instead of the filesystem.
use fs_btrfs::fs::Filesystem;
use fs_core::{BlockDevice, BlockRead};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
struct Dev(Mutex<std::fs::File>, u64, bool);
impl BlockRead for Dev {
    fn read_at(&self, off: u64, buf: &mut [u8]) -> fs_core::error::Result<()> {
        let mut f = self.0.lock().unwrap();
        f.seek(SeekFrom::Start(off))?;
        f.read_exact(buf)?;
        Ok(())
    }
    fn size_bytes(&self) -> u64 { self.1 }
}
impl BlockDevice for Dev {
    fn write_at(&self, off: u64, buf: &[u8]) -> fs_core::error::Result<()> {
        let mut f = self.0.lock().unwrap();
        f.seek(SeekFrom::Start(off))?;
        f.write_all(buf)?;
        Ok(())
    }
    fn flush(&self) -> fs_core::error::Result<()> { self.0.lock().unwrap().sync_all()?; Ok(()) }
    fn is_writable(&self) -> bool { self.2 }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let write = std::env::var("WRITE").is_ok();
    let f = std::fs::OpenOptions::new().read(true).write(write).open(&a[1]).unwrap();
    let len = f.metadata().unwrap().len();
    let dev = Arc::new(Dev(Mutex::new(f), len, write));
    let mut fs = if write {
        Filesystem::mount_rw(dev as Arc<dyn BlockDevice>).expect("mount_rw")
    } else {
        Filesystem::mount(dev as Arc<dyn BlockRead>).expect("mount")
    };
    if std::env::var("CHECK").is_ok() {
        let r = fs_btrfs::check::check(&fs);
        println!("C clean={} blocks={} inodes={}", r.is_clean(), r.tree_blocks, r.inodes);
        for f in &r.findings { println!("C {:?} {}", f.tree, f.what); }
    }
    for e in fs.read_dir(256).expect("readdir") { println!("D {}", String::from_utf8_lossy(&e.name)); }
    for p in a[3..].iter() {
        match fs.read_path(p) {
            Ok(d) => { std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap(); println!("R {p} {}", d.len()) }
            Err(e) => println!("E {p} {e:?}"),
        }
    }
    if write {
        let p = &a[3];
        let ino = fs.lookup_path(p).expect("lookup").ino;
        match fs.write(ino, 0, b"NIFE") {
            Ok(n) => println!("W {p} {n}"),
            Err(e) => println!("WE {p} {e}"),
        }
        let d = fs.read_path(p).unwrap();
        std::fs::write(format!("{}/readback", a[2]), &d).unwrap();
        println!("RB {p} {}", d.len());
    }
}
