// btrfsutils' clean-room write path for an unmounted btrfs. Creates /nife.txt in the top-level
// subvolume by hand (inode, directory entry, data, commit), which is the whole of what this crate
// offers: a tree-mutation library, with the POSIX layer left to its caller. The inode number and
// directory index are fixed large values because the crate has no allocator for either.
use btrfs_disk::items::Timespec;
use btrfs_transaction::{filesystem::Filesystem, inode::InodeArgs, transaction::Transaction};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = std::fs::OpenOptions::new().read(true).write(true).open(&a[1]).unwrap();
    let mut fs = Filesystem::open(f).expect("open");
    let mut tx = Transaction::start(&mut fs).expect("start");
    let data = b"written by a nife probe\n";
    let ino = 1_000_000u64;
    let t = Timespec { sec: 1_791_331_200, nsec: 0 };
    let mut args = InodeArgs::new(fs.generation, 0o100644);
    args.size = data.len() as u64;
    args.atime = t; args.ctime = t; args.mtime = t; args.otime = t;
    tx.create_inode(&mut fs, 5, ino, &args).expect("create_inode");
    tx.link_dir_entry(&mut fs, 5, 256, ino, b"nife.txt", 1, 1_000_000, t).expect("link");
    tx.write_file_data(&mut fs, 5, ino, 0, data, false, None).expect("data");
    tx.commit(&mut fs).expect("commit");
    println!("W ok");
}
