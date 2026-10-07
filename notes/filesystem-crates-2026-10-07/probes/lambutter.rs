// Read-only btrfs. Reads two paths and lists the root.
use lambutter::{Btrfs, Path};
struct F(std::fs::File);
impl lambutter::BlockRead for F {
    type Error = std::io::Error;
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<(), Self::Error> {
        use std::io::{Read, Seek, SeekFrom};
        self.0.seek(SeekFrom::Start(off))?;
        self.0.read_exact(buf)
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = std::fs::File::open(&a[1]).unwrap();
    let len = f.metadata().unwrap().len();
    let mut fs = Btrfs::open(F(f), len).expect("open");
    for e in fs.read_dir(Path::new(b"/").unwrap()).expect("readdir") {
        println!("D {}", String::from_utf8_lossy(&e.name));
    }
    for p in a[3..].iter() {
        let abs = format!("/{p}");
        match fs.read_file(Path::new(abs.as_bytes()).unwrap()) {
            Ok(d) => { std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap(); println!("R {p} {}", d.len()) }
            Err(e) => println!("E {p} {e:?}"),
        }
    }
}
