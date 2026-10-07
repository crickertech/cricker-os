// ext4 read-write (ext4_rs). 4 KiB blocks only. Reads the named paths; with WRITE set, creates
// a file in the root, writes it and reads it back through the same crate.
use ext4_rs::*;
// The crate keeps its constants private (`mod ext4_defs`), so the root inode and the mode are
// written as numbers.
const ROOT_INODE: u32 = 2;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
struct Dev(Mutex<std::fs::File>);
impl BlockDevice for Dev {
    fn read_offset(&self, off: usize) -> Vec<u8> {
        let mut f = self.0.lock().unwrap();
        let mut buf = vec![0u8; BLOCK_SIZE];
        f.seek(SeekFrom::Start(off as u64)).unwrap();
        f.read_exact(&mut buf).unwrap();
        buf
    }
    fn write_offset(&self, off: usize, data: &[u8]) {
        let mut f = self.0.lock().unwrap();
        f.seek(SeekFrom::Start(off as u64)).unwrap();
        f.write_all(data).unwrap();
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let write = std::env::var("WRITE").is_ok();
    let f = std::fs::OpenOptions::new().read(true).write(write).open(&a[1]).unwrap();
    let ext4 = Ext4::open(Arc::new(Dev(Mutex::new(f))));
    for e in ext4.dir_get_entries(ROOT_INODE) { println!("D {}", e.get_name()); }
    for p in a[3..].iter() {
        match ext4.generic_open(p, &mut 2, false, 0, &mut 0) {
            Ok(ino) => {
                let size = ext4.get_inode_ref(ino).inode.size() as usize;
                let mut d = vec![0u8; size];
                let n = ext4.read_at(ino, 0, &mut d).unwrap();
                d.truncate(n);
                std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap();
                println!("R {p} {}", d.len());
            }
            Err(e) => println!("E {p} {e:?}"),
        }
    }
    if write {
        let mode = 0o100644;
        let r = ext4.create(ROOT_INODE, "nife.txt", mode).expect("create");
        ext4.write_at(r.inode_num, 0, b"written by a nife probe\n").expect("write");
        println!("W ok");
        let mut d = vec![0u8; 24];
        let n = ext4.read_at(r.inode_num, 0, &mut d).unwrap();
        std::fs::write(format!("{}/readback", a[2]), &d[..n]).unwrap();
        println!("RB {n}");
    }
}
