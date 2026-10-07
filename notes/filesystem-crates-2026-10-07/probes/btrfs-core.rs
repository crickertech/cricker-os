// Forensic btrfs reader over a whole image held in memory.
use btrfs_core::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let img = std::fs::read(&a[1]).unwrap();
    let sb = Superblock::parse(&img[65536..65536 + 4096]).expect("superblock");
    let mut map = ChunkMap::new();
    match read_node(&img, &sb, &map, sb.chunk_root) { Ok(n) => map.add_from_node(&n), Err(e) => println!("E chunk {e:?}") }
    for p in a[3..].iter() {
        match read_by_path_content(&img, &sb, &map, p) {
            Ok(d) => { std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap(); println!("R {p} {}", d.len()) }
            Err(e) => println!("E {p} {e:?}"),
        }
    }
}
