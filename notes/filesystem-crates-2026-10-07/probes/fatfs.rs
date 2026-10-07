// FAT12/16/32 read-write (rust-fatfs 0.3). Reads the named paths; with WRITE set, creates a
// directory and a long-named file, remounts and reads the file back.
use std::io::{Read, Write};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let write = std::env::var("WRITE").is_ok();
    let f = std::fs::OpenOptions::new().read(true).write(write).open(&a[1]).unwrap();
    {
        let fs = fatfs::FileSystem::new(f, fatfs::FsOptions::new()).expect("mount");
        {
        let root = fs.root_dir();
        for e in root.iter() { println!("D {}", e.unwrap().file_name()); }
        for p in a[3..].iter() {
            let mut d = Vec::new();
            match root.open_file(p) {
                Ok(mut h) => { h.read_to_end(&mut d).unwrap(); std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap(); println!("R {p} {}", d.len()) }
                Err(e) => println!("E {p} {e:?}"),
            }
        }
        if write {
            root.create_dir("nife dir").expect("mkdir");
            let mut h = root.create_file("nife dir/Written By Nife.txt").expect("create");
            h.write_all(b"written by a nife probe\n").unwrap();
            h.flush().unwrap();
            println!("W ok");
        }
        }
        fs.unmount().expect("unmount");
    }
    if write {
        let f = std::fs::File::open(&a[1]).unwrap();
        let fs = fatfs::FileSystem::new(f, fatfs::FsOptions::new()).expect("remount");
        let mut d = Vec::new();
        fs.root_dir().open_file("nife dir/Written By Nife.txt").unwrap().read_to_end(&mut d).unwrap();
        std::fs::write(format!("{}/readback", a[2]), &d).unwrap();
        println!("RB {}", d.len());
    }
}
