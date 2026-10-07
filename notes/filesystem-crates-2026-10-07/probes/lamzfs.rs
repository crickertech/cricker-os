// Read-only ZFS (single, mirror, raidz1). One pool member; the dataset comes from DATASET
// (slash-separated, empty for the pool's root dataset).
use lamzfs::{BlockRead, PoolMember, Zfs};
struct F(std::fs::File);
impl BlockRead for F {
    type Error = std::io::Error;
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<(), Self::Error> {
        use std::io::{Read, Seek, SeekFrom};
        self.0.seek(SeekFrom::Start(off))?;
        self.0.read_exact(buf)
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ds_s = std::env::var("DATASET").unwrap_or_default();
    let ds: Vec<&str> = ds_s.split('/').filter(|s| !s.is_empty()).collect();
    let f = std::fs::File::open(&a[1]).unwrap();
    let len = f.metadata().unwrap().len();
    let mut zfs = Zfs::import(vec![PoolMember { reader: F(f), device_size_bytes: len }]).expect("import");
    println!("P {}", zfs.pool_name());
    for e in zfs.read_dir(&ds, &[]).expect("readdir") { println!("D {}", e.name); }
    for p in a[3..].iter() {
        let parts: Vec<&str> = p.split('/').collect();
        match zfs.read(&ds, &parts) {
            Ok(d) => { std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap(); println!("R {p} {}", d.len()) }
            Err(e) => println!("E {p} {e:?}"),
        }
    }
}
