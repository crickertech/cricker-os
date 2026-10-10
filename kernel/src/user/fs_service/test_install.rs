//! **The kernel test harness installs a package as an installer would** (milestone 152 (durable
//! delegation)'s Fork 8 D). Moved out of `fs_service.rs` unchanged, apart from the activation set's
//! manager column, by milestone 809 (the package client becomes a program), when that file reached
//! §266 (a Rust source file stays under 2,000 lines)'s ratchet.

use super::*;

/// **Install `bytes` as the one program of package `name` at `version`, as an installer would**
/// (milestone 152 (durable delegation), Fork 8 D, which runs a scheduled job from the store): the
/// bytes at `packages/<name>/<version>/<program>`, and a generation `1` in `activation/` naming them
/// with their digest, made live by `current`. Anything already there is overwritten, and a
/// generation this writes replaces the whole table, which no test here needs more of.
pub fn install_for_test(program: &str, name: &str, version: &str, bytes: &[u8]) {
    use filesystem_protocol::{PAGE, dir as rights, fs};
    let (fs_ep, fs_page_frame) = root_directory(
        blk_server_image(),
        crate::user::program("redoxfs_server").expect("no redoxfs_server in the initrd"),
    )
    .expect("wired() already brought the file service up");
    // SAFETY: as in `set_file`: the file service's own shared page, idle between logins.
    let page = unsafe {
        core::slice::from_raw_parts_mut(mmu::phys_to_virt(fs_page_frame) as *mut u8, PAGE)
    };
    let call = |w: [u64; 2]| crate::sched::ipc_call(fs_ep, w)[0] as i64;
    let named = |page: &mut [u8], verb: u64, at: u64, name: &str, w1: u64| {
        page[..name.len()].copy_from_slice(name.as_bytes());
        call([fs::req(verb, at, name.len() as u64), w1])
    };
    // A directory under `at`, made if it is not there.
    let dir = |page: &mut [u8], at: u64, name: &str| {
        let made = named(page, fs::MKDIR, at, name, rights::ALL);
        if made >= 0 {
            return made as u64;
        }
        let h = named(page, fs::OPENDIR, at, name, rights::ALL);
        assert!(h >= 0, "could not make or open {name} ({h})");
        h as u64
    };
    // `contents` as the whole of `file` under `at`, a page at a time.
    let put = |page: &mut [u8], at: u64, file: &str, contents: &[u8]| {
        let mut h = named(page, fs::OPEN, at, file, 0);
        if h < 0 {
            h = named(page, fs::CREATE, at, file, 0);
        }
        assert!(h >= 0, "could not open or create {file} ({h})");
        let h = h as u64;
        assert_eq!(
            call([fs::req(fs::TRUNCATE, h, 0), 0]),
            0,
            "could not truncate {file}"
        );
        for (k, chunk) in contents.chunks(PAGE).enumerate() {
            page[..chunk.len()].copy_from_slice(chunk);
            let wrote = call([fs::req(fs::WRITE, h, chunk.len() as u64), (k * PAGE) as u64]);
            assert_eq!(wrote, chunk.len() as i64, "short write of {file}");
        }
        call([fs::req(fs::CLOSE, h, 0), 0]);
    };
    let packages = dir(page, fs::ROOT, activation_set::PACKAGES);
    let n = dir(page, packages, name);
    let v = dir(page, n, version);
    put(page, v, program, bytes);
    for h in [v, n, packages] {
        call([fs::req(fs::CLOSE, h, 0), 0]);
    }

    // The row and its pointer (milestone 614 (two installed versions of one program)): the bytes'
    // path is its package and version columns, and its manager is `jig`, as milestone 809 (the
    // package client becomes a program) records one.
    let entry = activation_set::Entry {
        program,
        version,
        package: name,
        digest: measured_boot::sha256(bytes),
        manager: "jig",
    };
    let mut table = [0u8; 512];
    let len = activation_set::with_entry(
        "",
        &entry,
        crate::user::program(program).is_some(),
        &mut table,
    )
    .expect("one entry fits");
    let mut current = [0u8; 16];
    let current_len = activation_set::format_current(1, &mut current).expect("fits");
    let mut digits = [0u8; 10];
    let first = activation_set::generation_name(1, &mut digits);
    let activation = dir(page, fs::ROOT, activation_set::DIRECTORY);
    put(page, activation, first, &table[..len]);
    put(
        page,
        activation,
        activation_set::CURRENT,
        &current[..current_len],
    );
    call([fs::req(fs::CLOSE, activation, 0), 0]);
}
