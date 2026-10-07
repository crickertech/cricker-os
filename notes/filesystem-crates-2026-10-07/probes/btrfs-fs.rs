// btrfsutils' high-level read API, which runs every operation through tokio's blocking pool.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
    rt.block_on(async {
        let f = std::fs::File::open(&a[1]).unwrap();
        let fs = btrfs_fs::Filesystem::open(f).expect("open");
        let root = fs.root();
        for e in fs.readdir(root, 0).await.expect("readdir") { println!("D {}", String::from_utf8_lossy(&e.name)); }
        for p in a[3..].iter() {
            let mut ino = root;
            let mut ok = true;
            for part in p.split('/') {
                match fs.lookup(ino, part.as_bytes()).await { Ok(Some((i, _))) => ino = i, _ => { ok = false; break } }
            }
            if !ok { println!("E {p} lookup"); continue }
            let d = fs.read(ino, 0, 1 << 24).await.expect("read");
            std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap();
            println!("R {p} {}", d.len());
        }
    });
}
