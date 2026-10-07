//! **The host side of the search measurement in milestone 121 (`ripgrep` on nife)**: stage the
//! priced tree, then run unmodified `ripgrep` over it exactly as nife's kernel harness and bench boot do, and print what
//! `rg --stats` says it spent.
//!
//! ```text
//! cargo run --release -p walk_pricing --example rg_host -- <path to rg> [runs]
//! ```
//!
//! The command line is `filesystem_protocol::fixture::walk::RG_SEARCH`, split into words by
//! `grant_plan::each_word`, the function that builds nife's argv, so both sides hear the same
//! words. It runs with the staged tree as its working directory. Its output goes to a pipe this program drains, which is
//! the nearest thing on a host to nife's byte sink. The figure compared is ripgrep's own
//! `seconds` line: it starts after argument parsing and ends after the last file, so neither side's
//! process creation or image loading is in it. The spawn-to-exit time is printed beside it for
//! what it is, a different and less comparable number.
//!
//! `bench/host/run_linux_rg.sh` builds this static for musl and boots it as PID 1 on the machine
//! nife's bench boot uses, with a static Linux `rg` beside it in the initramfs. On macOS it is a
//! reference point only (APFS on NVMe, a different kernel and a different `rg` build).
//!
//! # BUGS
//!
//! - The tree is staged on whatever filesystem `DIR` is on, which under the Linux script is tmpfs.
//!   Every timed run after the first is warm on both sides, which is the figure compared.

use std::process::Command;
use std::time::Instant;

use filesystem_protocol::fixture::walk as tree;

/// `rg --stats`'s last line, `<seconds> seconds`, in nanoseconds.
fn rg_seconds(text: &str) -> Option<u128> {
    let line = text.lines().rev().find(|l| l.ends_with(" seconds"))?;
    let secs = line.strip_suffix(" seconds")?;
    let (whole, frac) = secs.split_once('.')?;
    let frac = format!("{frac:0<9}");
    Some(whole.parse::<u128>().ok()? * 1_000_000_000 + frac[..9].parse::<u128>().ok()?)
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let rg = args
        .next()
        .expect("usage: rg_host <path to rg> [runs] [dir]");
    let runs: usize = args
        .next()
        .and_then(|r| r.to_str().and_then(|r| r.parse().ok()))
        .unwrap_or(5);
    let dir = args
        .next()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join(format!("rg-walk-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&dir);
    walk_pricing::stage(&dir).expect("could not stage the priced tree");
    let mut words: Vec<Vec<u8>> = Vec::new();
    grant_plan::each_word(tree::RG_SEARCH.as_bytes(), &mut |w| {
        words.push(w.to_vec());
        Ok(())
    })
    .expect("RG_SEARCH is a line the shell takes");
    let words: Vec<String> = words
        .into_iter()
        .map(|w| String::from_utf8(w).expect("ASCII"))
        .collect();

    // One untimed run first, so every timed one is warm, as nife's bench row is.
    for run in 0..=runs {
        let t = Instant::now();
        // stdin is inherited rather than `output()`'s default of /dev/null, for two reasons. The
        // Linux run is PID 1 on an initramfs with no /dev, so opening /dev/null fails the spawn.
        // And `rg` given no path searches stdin when stdin is a file or a pipe; inheriting the
        // console (or a terminal) keeps it walking its working directory, as nife's does.
        let out = Command::new(&rg)
            .args(&words[1..])
            .current_dir(&dir)
            .stdin(std::process::Stdio::inherit())
            .output()
            .unwrap_or_else(|e| {
                panic!(
                    "could not run {rg:?} in {dir:?} (it exists: {}, the directory: {}): {e}",
                    std::path::Path::new(&rg).exists(),
                    dir.exists()
                )
            });
        let wall = t.elapsed();
        let text = String::from_utf8_lossy(&out.stdout);
        if run == 0 {
            // The untimed run's whole transcript tail, so a reader sees the counts nife asserts.
            println!(
                "rg exit {:?}, {} bytes out",
                out.status.code(),
                out.stdout.len()
            );
            for line in text
                .lines()
                .rev()
                .take(9)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                println!("rg {line}");
            }
            continue;
        }
        let Some(ns) = rg_seconds(&text) else {
            println!("rg printed no stats");
            continue;
        };
        println!(
            "rg search {ns} ns (rg's own figure), spawn to exit {} ns",
            wall.as_nanos()
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
