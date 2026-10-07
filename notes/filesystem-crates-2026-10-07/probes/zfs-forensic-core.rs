// Forensic ZFS reader over a whole image in memory; reads the pool's root dataset only.
use zfs_core::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let img = std::fs::read(&a[1]).unwrap();
    let label = VdevLabel::parse(&img[..LABEL_SIZE]).expect("label");
    let mos_block = read_block(&img, &label.active_uberblock.rootbp_full()).expect("mos");
    let mos = ObjsetPhys::parse(&mos_block.data, label.active_uberblock.endian).expect("mos objset");
    let Some(zpl) = zpl_objset(&img, &mos) else { println!("E no root dataset"); return };
    for p in a[3..].iter() {
        match zpl_read_path(&img, &zpl, p) {
            Ok(d) => { std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap(); println!("R {p} {}", d.len()) }
            Err(e) => println!("E {p} {e:?}"),
        }
    }
}
