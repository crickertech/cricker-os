// FAT12/16/32 read-write (lamfat 0.4, a republish of rust-fatfs master). Reads the named paths; with WRITE set, creates a
// directory and a long-named file, remounts and reads the file back.
use fatfs::{Read, Write};
extern crate lamfat as fatfs;
// The 0.4 line has its own Read trait, without read_to_end.
fn slurp<R: Read>(h: &mut R, d: &mut Vec<u8>) {
    let mut buf = [0u8; 4096];
    loop { let n = h.read(&mut buf).ok().unwrap(); if n == 0 { break } d.extend_from_slice(&buf[..n]); }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let write = std::env::var("WRITE").is_ok();
    let f = std::fs::OpenOptions::new().read(true).write(write).open(&a[1]).unwrap();
    {
        let fs = fatfs::FileSystem::new(fatfs::StdIoWrapper::from(f), fatfs::FsOptions::new()).expect("mount");
        {
        let root = fs.root_dir();
        for e in root.iter() { println!("D {}", e.unwrap().file_name()); }
        for p in a[3..].iter() {
            let mut d = Vec::new();
            match root.open_file(p) {
                Ok(mut h) => { slurp(&mut h, &mut d); std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap(); println!("R {p} {}", d.len()) }
                Err(e) => println!("E {p} {e:?}"),
            }
        }
        if write {
            root.create_dir("nife dir").expect("mkdir");
            let mut h = root.create_file("nife dir/Written By Nife.txt").expect("create");
            h.write_all(b"written by a nife probe\n").ok().unwrap();
            h.flush().ok().unwrap();
            println!("W ok");
        }
        }
        fs.unmount().expect("unmount");
    }
    if write {
        let f = std::fs::File::open(&a[1]).unwrap();
        let fs = fatfs::FileSystem::new(fatfs::StdIoWrapper::from(f), fatfs::FsOptions::new()).expect("remount");
        let mut d = Vec::new();
        slurp(&mut fs.root_dir().open_file("nife dir/Written By Nife.txt").unwrap(), &mut d);
        std::fs::write(format!("{}/readback", a[2]), &d).unwrap();
        println!("RB {}", d.len());
    }
}
