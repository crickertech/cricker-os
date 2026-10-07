// ferrosys's family-detecting reader: btrfs, FAT, exFAT or ext, whichever the image is. Walks the
// tree and extracts every regular file whose path ends with a name given on the command line.
use ferrosys::{FsReader, FsTree, NodeKind, TreeEntry, TreeError};
fn dump<T: FsTree>(t: &mut T, out: &str, want: &[String]) where T::Node: Clone {
    let mut files: Vec<(String, T::Node)> = Vec::new();
    t.walk_tree::<TreeError, _>(|_, e: TreeEntry<T::Node>| {
        let p = String::from_utf8_lossy(&e.path).into_owned();
        println!("D {p} {:?}", e.kind);
        if matches!(e.kind, NodeKind::File { .. }) { files.push((p, e.node.clone())); }
        Ok(())
    }).expect("walk");
    for (p, n) in files {
        let Some(w) = want.iter().find(|w| p.trim_start_matches('/') == w.as_str()) else { continue };
        let mut d = Vec::new();
        let mut buf = vec![0u8; 65536];
        loop {
            let k = t.read_bytes(&n, d.len() as u64, &mut buf).expect("read");
            if k == 0 { break; }
            d.extend_from_slice(&buf[..k]);
        }
        std::fs::write(format!("{out}/{}", w.replace('/', "_")), &d).unwrap();
        println!("R {w} {}", d.len());
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = std::io::BufReader::new(std::fs::File::open(&a[1]).unwrap());
    match ferrosys::open(f).expect("open") {
        FsReader::Btrfs(mut r) => dump(&mut r, &a[2], &a[3..]),
        FsReader::Fat(mut r) => dump(&mut r, &a[2], &a[3..]),
        FsReader::ExFat(mut r) => dump(&mut r, &a[2], &a[3..]),
        FsReader::Ext(mut r) => dump(&mut r, &a[2], &a[3..]),
        #[allow(unreachable_patterns)]
        _ => println!("E unknown family"),
    }
}
