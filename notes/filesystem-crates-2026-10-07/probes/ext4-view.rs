// Read-only ext2/3/4.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let fs = ext4_view::Ext4::load(Box::new(std::fs::read(&a[1]).unwrap())).expect("load");
    for e in fs.read_dir("/").expect("readdir") { println!("D {}", e.unwrap().path().display()); }
    for p in a[3..].iter() {
        match fs.read(format!("/{p}").as_str()) {
            Ok(d) => { std::fs::write(format!("{}/{}", a[2], p.replace('/', "_")), &d).unwrap(); println!("R {p} {}", d.len()) }
            Err(e) => println!("E {p} {e:?}"),
        }
    }
}
