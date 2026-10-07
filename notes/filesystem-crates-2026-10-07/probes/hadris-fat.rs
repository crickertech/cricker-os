// FAT12/16/32 read-write (hadris-fat 3.0 release candidate). The whole image is held in memory
// (`MemDevice`), as the crate's own examples do. With WRITE set, creates a directory and a
// long-named file, unmounts, writes the image back, remounts and reads the file back.
use hadris_fat::sync::FatFs;
use hadris_fs::sync::{FileSystem, Volume};
use hadris_fs::{MountOptions, OpenOptions};
use hadris_storage::{BlockSize, MemDevice};
fn mount(img: Vec<u8>) -> Volume<FatFs<MemDevice<Vec<u8>>>> {
    Volume::new(FatFs::mount(MemDevice::new(img, BlockSize::new(512).unwrap()), MountOptions::new()).expect("mount"))
}
fn read_all(vol: &Volume<FatFs<MemDevice<Vec<u8>>>>, p: &str) -> Result<Vec<u8>, String> {
    let mut h = vol.open(p, OpenOptions::new().read()).map_err(|e| format!("{e:?}"))?;
    let mut d = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = h.read(&mut buf).map_err(|e| format!("{e:?}"))?;
        if n == 0 { break; }
        d.extend_from_slice(&buf[..n]);
    }
    Ok(d)
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let write = std::env::var("WRITE").is_ok();
    let vol = mount(std::fs::read(&a[1]).unwrap());
    for e in vol.read_dir("/").expect("readdir") { println!("D {:?}", e.unwrap().name()); }
    for p in a[3..].iter() {
        match read_all(&vol, &format!("/{p}")) {
            Ok(d) => { std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap(); println!("R {p} {}", d.len()) }
            Err(e) => println!("E {p} {e}"),
        }
    }
    if write {
        vol.create_dir_all("/nife dir").expect("mkdir");
        let mut h = vol.open("/nife dir/Written By Nife.txt", OpenOptions::new().write().create()).expect("create");
        h.write(b"written by a nife probe\n").unwrap();
        h.close().unwrap();
        vol.lock().sync().unwrap();
        println!("W ok");
        let dev = vol.into_inner().ok().expect("sole owner").unmount().expect("unmount");
        let img = dev.into_inner();
        std::fs::write(&a[1], &img).unwrap();
        let vol = mount(img);
        let d = read_all(&vol, "/nife dir/Written By Nife.txt").unwrap();
        std::fs::write(format!("{}/readback", a[2]), &d).unwrap();
        println!("RB {}", d.len());
    }
}
