// FAT16/32 read-write over a BlockDevice, MBR partition required, 8.3 names only. Reads the
// named 8.3 paths from partition 0; with WRITE set, writes NIFE.TXT, closes, reopens, reads back.
use embedded_sdmmc::{Block, BlockCount, BlockDevice, BlockIdx, Mode, TimeSource, Timestamp, VolumeIdx, VolumeManager};
use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom, Write};
struct Dev(RefCell<std::fs::File>, u64);
impl BlockDevice for Dev {
    type Error = std::io::Error;
    fn read(&self, blocks: &mut [Block], start: BlockIdx) -> Result<(), Self::Error> {
        let mut f = self.0.borrow_mut();
        f.seek(SeekFrom::Start(start.0 as u64 * 512))?;
        for b in blocks { f.read_exact(&mut b.contents)?; }
        Ok(())
    }
    fn write(&self, blocks: &[Block], start: BlockIdx) -> Result<(), Self::Error> {
        let mut f = self.0.borrow_mut();
        f.seek(SeekFrom::Start(start.0 as u64 * 512))?;
        for b in blocks { f.write_all(&b.contents)?; }
        Ok(())
    }
    fn num_blocks(&self) -> Result<BlockCount, Self::Error> { Ok(BlockCount((self.1 / 512) as u32)) }
}
struct Clock;
impl TimeSource for Clock {
    fn get_timestamp(&self) -> Timestamp { Timestamp { year_since_1970: 56, zero_indexed_month: 9, zero_indexed_day: 6, hours: 0, minutes: 0, seconds: 0 } }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let write = std::env::var("WRITE").is_ok();
    let f = std::fs::OpenOptions::new().read(true).write(write).open(&a[1]).unwrap();
    let len = f.metadata().unwrap().len();
    let vm: VolumeManager<_, _> = VolumeManager::new(Dev(RefCell::new(f), len), Clock);
    let vol = vm.open_volume(VolumeIdx(0)).expect("volume");
    let root = vol.open_root_dir().expect("root");
    root.iterate_dir(|e| { println!("D {}", e.name); core::ops::ControlFlow::Continue(()) }).unwrap();
    for p in a[3..].iter() {
        match root.open_file_in_dir(p.as_str(), Mode::ReadOnly) {
            Ok(h) => {
                let mut d = Vec::new();
                let mut buf = [0u8; 512];
                while !h.is_eof() { let n = h.read(&mut buf).unwrap(); d.extend_from_slice(&buf[..n]); }
                std::fs::write(format!("{}/{}", a[2], p), &d).unwrap();
                println!("R {p} {}", d.len());
            }
            Err(e) => println!("E {p} {e:?}"),
        }
    }
    if write {
        let h = root.open_file_in_dir("NIFE.TXT", Mode::ReadWriteCreateOrTruncate).expect("create");
        h.write(b"written by a nife probe\n").unwrap();
        h.close().unwrap();
        let h = root.open_file_in_dir("NIFE.TXT", Mode::ReadOnly).unwrap();
        let mut d = Vec::new();
        let mut buf = [0u8; 512];
        while !h.is_eof() { let n = h.read(&mut buf).unwrap(); d.extend_from_slice(&buf[..n]); }
        std::fs::write(format!("{}/readback", a[2]), &d).unwrap();
        println!("W ok");
        println!("RB {}", d.len());
    }
}
