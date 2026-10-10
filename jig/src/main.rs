//! **`jig`, the package manager** (milestone 809 (the package client becomes a program)).
//!
//! ```text
//! jig list                     the packages this image vouches for, from its catalog
//! jig install <path>           install a package file already on this system
//! jig install <name>[@<ver>]   fetch a package by name and install it
//! jig remove <program>         a new generation without the program
//! jig rollback                 the generation below the live one becomes live
//! ```
//!
//! The verbs are apt's where apt has one (calef, 2026-10-10 (UTC): "Lets use apt's verbs.";
//! design/naming/command-line-rulings.md). A word with a `/` in it is a file, by the prompt's rule
//! for a command word (DECISIONS §219 (how the shell names an installed program to the spawner)).
//!
//! The progenitor stays the installer (DECISIONS §208 (installing a package is granting it, and the
//! activation set is versioned)) and checks every package against its own copy of the catalog.
//! This is the client. It asks on an installer endpoint that serves install, remove and rollback
//! and nothing else (DECISIONS §270 (a package manager holds an installer endpoint, not the spawn
//! endpoint)), with `spawnproto`'s activation request unchanged. It fetches a package itself, so
//! the progenitor parses no network input, and checks it before sending: it must be the package
//! asked for, and the catalog must vouch for its digest (`package_archive::installable_as`).
//!
//! The package is read into one run of this program's pages and handed over a page at a time with
//! `abi::page_frame::SLICE`, so the buffer and the frames are the same memory.
//!
//! notes/packages/jig.md has what each verb holds, how an install travels, and the BUGS.
//!
//! Name: ratified 2026-10-06 (calef). See `grant_plan::Prog::Jig`.

use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::process::ExitCode;

use grant_plan::spawnproto::{self, Activation, ActivationStatus as S};
use grant_plan::{CATALOG_SLOT, INSTALLER_REPLY_SLOT, INSTALLER_SLOT};
use socket_protocol::fixture::{PACKAGE_PEER_HOST, PACKAGE_PEER_IP, PACKAGE_PEER_PORT};
use user_mode_runtime as rt;

// **What these bytes ask for when they are run by path, as installed bytes** (milestone 597 (a
// program carries its manifest in an ELF note)): `jig`'s row without the three grants no image may
// hold. The image's own `jig` is endowed from `grant_plan`'s table and never reads this; a copy run
// from a file is, and it finds no installer endpoint and says so, which is how `script/swish-check`
// shows a program without the grant is refused. The note cannot carry `installer` or `catalog`
// (`manifest_note::encode` refuses both at compile time), and a `std` image cannot carry the network.
manifest_note::carry!(grant_plan::Manifest {
    network: false,
    installer: false,
    catalog: false,
    ..grant_plan::Prog::Jig.manifest()
});

/// **Where the staging run is mapped**: one package, read here and handed on a page at a time. A
/// window of this program's own, so a pair page (`address_space_map`'s rule 7), clear of everything
/// a loader maps into a `std` program. Four MiB, `spawnproto::IMAGE_MAX_PAGES` pages.
const STAGING_VA: u64 = address_space_map::pair_page(0x0400_0000);

/// This machine's architecture, as a package stem spells it.
const ARCHITECTURE: &str = std::env::consts::ARCH;

const USAGE: &str = "  jig list | jig install <file or name> | jig remove <program> | jig rollback";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["list"] => list(),
        ["install", path] if path.contains('/') => install_file(path),
        ["install", name] => install_name(name),
        ["remove", program] => remove(program),
        ["rollback"] => rollback(),
        _ => {
            println!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// **The image's catalog**, as this program was handed it: the text before the page's first zero
/// byte. `None` for a `jig` that was not granted one, which is told so rather than faulting on an
/// unmapped page.
fn catalog() -> Option<&'static str> {
    if !rt::is_granted(CATALOG_SLOT) {
        return None;
    }
    // SAFETY: the progenitor maps the catalog's page read-only at `CATALOG_PAGE` whenever it
    // places the page's capability at `CATALOG_SLOT` (`system_initializer`'s `StdLayout`), and the
    // mapping lives as long as this process.
    let page = unsafe {
        std::slice::from_raw_parts(
            std_runtime_protocol::CATALOG_PAGE as *const u8,
            spawnproto::IMAGE_PAGE as usize,
        )
    };
    let end = page.iter().position(|&b| b == 0).unwrap_or(page.len());
    std::str::from_utf8(&page[..end]).ok()
}

/// **`jig list`**: every package this image's catalog vouches for on this architecture, one per
/// line, as `<name> <version>`.
fn list() -> ExitCode {
    let Some(catalog) = catalog() else {
        println!("  this jig holds no catalog; only the image's own jig is handed one");
        return ExitCode::FAILURE;
    };
    println!("  packages this image vouches for (its catalog; no index yet):");
    let suffix = format!("-{ARCHITECTURE}");
    for line in catalog.lines() {
        let Some(stem) = line.split(' ').next() else {
            continue;
        };
        let Some(rest) = stem.strip_suffix(suffix.as_str()) else {
            continue;
        };
        if let Some((name, version)) = rest.rsplit_once('-') {
            println!("  {name} {version}");
        }
    }
    ExitCode::SUCCESS
}

/// **A package's bytes, in a run of this program's own pages**, ready to hand on a page at a time.
struct Staged {
    /// The run's capability, with `GRANT`, which `SLICE` needs.
    run: u64,
    len: u64,
}

impl Staged {
    /// Retype a run for `len` bytes from this program's region and map it at [`STAGING_VA`].
    fn new(len: u64) -> Result<Staged, &'static str> {
        let pages = spawnproto::image_pages(len);
        if pages == 0 || pages > spawnproto::IMAGE_MAX_PAGES {
            return Err("that package is empty, or larger than a package may be (4 MiB)");
        }
        let run = rt::retype_page_frame_run(std_runtime_protocol::MEMORY_REGION_SLOT, pages);
        let Ok(run) = u64::try_from(run) else {
            return Err("this jig could not set aside the pages for that package");
        };
        if !rt::map_page_frame(
            run,
            STAGING_VA,
            true,
            std_runtime_protocol::MEMORY_REGION_SLOT,
        ) {
            rt::cap_delete(run);
            return Err("this jig could not map the pages for that package");
        }
        Ok(Staged { run, len })
    }

    fn bytes(&mut self) -> &mut [u8] {
        // SAFETY: `new` mapped `image_pages(len)` pages read/write at `STAGING_VA`, which nothing
        // else in this process uses, and `len` is at most that many pages.
        unsafe { std::slice::from_raw_parts_mut(STAGING_VA as *mut u8, self.len as usize) }
    }
}

/// **Send one activation request on the installer endpoint and read its one answer**: the frames
/// of `staged` for an install, the packed name for a removal, nothing for a rollback.
fn ask(verb: Activation, staged: Option<&Staged>, name: Option<&str>) -> (S, u64) {
    let len = staged.map_or(0, |s| s.len);
    let (w0, w1, w2) = spawnproto::activation_request(verb, len);
    rt::send(INSTALLER_SLOT, w0, w1, w2);
    if let Some(st) = staged {
        for i in 0..spawnproto::image_pages(st.len) {
            match u64::try_from(rt::slice_page_frame(st.run, i, 1)) {
                Ok(page) => {
                    rt::send_cap(INSTALLER_SLOT, page, abi::rights::READ, i);
                    rt::cap_delete(page);
                }
                // A plain message where a frame was promised keeps both sides in step: the
                // progenitor counts a missing frame and refuses the package.
                Err(_) => {
                    rt::send(INSTALLER_SLOT, 0, 0, 0);
                }
            }
        }
    }
    if let Some(name) = name {
        let (lo, hi) = filesystem_protocol::grant::pack_name(name.as_bytes());
        rt::send(INSTALLER_SLOT, lo, hi, name.len() as u64);
    }
    let (r0, r1, _) = rt::receive(INSTALLER_REPLY_SLOT);
    (S::from_word(r0), r1)
}

/// Whether this process was granted an installer endpoint; says why not when it was not.
fn holds_installer() -> bool {
    let held = rt::is_granted(INSTALLER_SLOT) && rt::is_granted(INSTALLER_REPLY_SLOT);
    if !held {
        println!("  this jig holds no installer endpoint; only the owner's console grants one");
    }
    held
}

/// **`jig install <path>`**: the file's bytes, sent as they are. The progenitor's catalog decides.
fn install_file(path: &str) -> ExitCode {
    if !holds_installer() {
        return ExitCode::FAILURE;
    }
    // `./` is how a person says "a file, here" (DECISIONS §219's rule for a word with a `/`); the
    // shell designated the name after it, so that is the name opened.
    let mut name = path;
    while let Some(rest) = name.strip_prefix("./") {
        name = rest;
    }
    let mut file = match std::fs::File::open(name) {
        Ok(f) => f,
        Err(e) => return say_io(path, e),
    };
    let len = match file.metadata() {
        Ok(m) => m.len(),
        Err(e) => return say_io(path, e),
    };
    let mut staged = match Staged::new(len) {
        Ok(s) => s,
        Err(why) => return refuse(why),
    };
    if let Err(e) = file.read_exact(staged.bytes()) {
        return say_io(path, e);
    }
    answer(
        Activation::Install,
        false,
        ask(Activation::Install, Some(&staged), None),
    )
}

/// **`jig install <name>`**: the catalog names the stem, the package source serves
/// `/<stem>.nifepkg`, and what arrives is checked before anything is sent: it must be the package
/// asked for, and the catalog must vouch for its digest.
fn install_name(name: &str) -> ExitCode {
    if !holds_installer() {
        return ExitCode::FAILURE;
    }
    let Some(catalog) = catalog() else {
        return refuse("this jig holds no catalog to look that name up in");
    };
    let stem = match package_archive::cataloged_stem(catalog, name, ARCHITECTURE) {
        Ok(stem) => stem,
        Err(package_archive::CatalogMiss::NoSuchPackage) => {
            return refuse("this image's catalog names no such package, so nothing was fetched");
        }
        Err(package_archive::CatalogMiss::SeveralVersions) => {
            return refuse(
                "this image's catalog vouches for several versions of that package; \
                 name one with <package>@<version>",
            );
        }
    };
    let mut staged = match fetch(stem) {
        Ok(s) => s,
        Err(why) => {
            println!("  {why}; nothing was installed");
            return ExitCode::FAILURE;
        }
    };
    if package_archive::installable_as(catalog, stem, staged.bytes()).is_err() {
        return refuse(
            "this image's catalog does not vouch for those bytes; nothing was sent to the installer",
        );
    }
    answer(
        Activation::Install,
        true,
        ask(Activation::Install, Some(&staged), None),
    )
}

/// **`GET /<stem>.nifepkg` from the package source**, into a staging run sized from the response's
/// declared length. `http_response` reads the head and refuses what it cannot read exactly; the
/// body goes straight into the run.
fn fetch(stem: &str) -> Result<Staged, &'static str> {
    const FAILED: &str = "the package source did not send a whole package";
    let mut tcp = TcpStream::connect((Ipv4Addr::from(PACKAGE_PEER_IP), PACKAGE_PEER_PORT))
        .map_err(|_| "the package source did not answer")?;
    let path = format!("/{stem}.nifepkg");
    let mut request = [0u8; 160];
    let n = http_response::get_request(PACKAGE_PEER_HOST, &path, &mut request).ok_or(FAILED)?;
    tcp.write_all(&request[..n]).map_err(|_| FAILED)?;
    let mut response = http_response::Response::new();
    let mut staged: Option<Staged> = None;
    let mut filled = 0usize;
    let mut buf = [0u8; 2048];
    while !response.is_complete() {
        let n = match tcp.read(&mut buf) {
            Ok(0) | Err(_) => return Err(FAILED),
            Ok(n) => n,
        };
        let body = response.feed(&buf[..n]).map_err(|_| FAILED)?;
        if response.status().is_some_and(|s| s != 200) {
            return Err(FAILED);
        }
        if staged.is_none()
            && let Some(len) = response.content_length()
        {
            staged = Some(Staged::new(len)?);
        }
        if !body.is_empty() {
            let st = staged.as_mut().ok_or(FAILED)?;
            // `feed` hands back no more body than the head declared, and the run was sized from
            // that declaration, so this stays inside it.
            let dest = st
                .bytes()
                .get_mut(filled..filled + body.len())
                .ok_or(FAILED)?;
            dest.copy_from_slice(body);
            filled += body.len();
        }
    }
    staged.ok_or(FAILED)
}

/// **`jig remove <program>`**, or `<program>@<version>` for one version.
fn remove(program: &str) -> ExitCode {
    if program.len() > filesystem_protocol::grant::MAX_NAME {
        println!("{USAGE}");
        return ExitCode::from(2);
    }
    if !holds_installer() {
        return ExitCode::FAILURE;
    }
    answer(
        Activation::Remove,
        false,
        ask(Activation::Remove, None, Some(program)),
    )
}

/// **`jig rollback`**: one generation back.
fn rollback() -> ExitCode {
    if !holds_installer() {
        return ExitCode::FAILURE;
    }
    answer(
        Activation::Rollback,
        false,
        ask(Activation::Rollback, None, None),
    )
}

/// Say a refusal this program made itself, sending nothing.
fn refuse(why: &str) -> ExitCode {
    println!("  refused: {why}");
    ExitCode::FAILURE
}

fn say_io(path: &str, e: std::io::Error) -> ExitCode {
    let why = match e.kind() {
        ErrorKind::Unsupported => "no directory was granted to read it from".to_string(),
        k => format!("{k:?}"),
    };
    println!("  could not read {path}: {why}");
    ExitCode::FAILURE
}

/// **What the progenitor's answer means**, in the words the shell's `package` builtin used, so a
/// person who learned those reads the same sentences. Every answer names the generation live
/// afterwards, because after a refusal that is still the fact a person needs.
fn answer(verb: Activation, fetched: bool, (status, live): (S, u64)) -> ExitCode {
    let said = match (status, verb) {
        (S::Done, Activation::Install) if fetched => "fetched and installed",
        (S::Done, Activation::Install) => "installed",
        (S::Done, Activation::Remove) => "removed",
        (S::Done, _) => "rolled back",
        (S::NotCataloged, _) => "refused: this image's catalog does not vouch for those bytes",
        (S::NoProgram, _) => "refused: that package carries no program named after it",
        (S::NotInstalled, _) => "refused: no program of that name is in the live generation",
        (S::NoEarlier, _) => "refused: there is no generation before the live one",
        (S::StoreFailed, _) => "could not write the activation set",
        (S::NameTaken, _) => {
            "refused: another installed package already provides a program of that name"
        }
        (S::ImageName, _) => {
            "refused: the image carries a program of that name; a new base updates it, not install"
        }
        (S::Ambiguous, _) => {
            "refused: several versions of that program are live and one holds the default; \
             name one with <program>@<version>"
        }
        (S::NotServedHere, _) => "refused: the installer endpoint does not take that request",
        (S::NotYours, _) => {
            "refused: another package manager installed that, and it is not jig's to undo"
        }
        (S::NotExecutable | S::Unknown, _) => "the progenitor could not take that request",
    };
    if live == 0 {
        println!("  {said}; nothing is installed");
    } else {
        println!("  {said}; generation {live} is live");
    }
    if status == S::Done {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
