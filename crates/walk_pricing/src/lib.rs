//! **What a directory walk costs, separated into the parts that cost it** (milestone 121 (`ripgrep` on
//! nife: enumeration as a capability), and what the walk costs).
//!
//! Every `read_dir` on nife is a message to a filesystem server, and every path is resolved one
//! component per message (`OPENDIR` per hop, then the verb, then a `CLOSE` per hop). A recursive
//! walk is therefore the workload that most directly prices the system's central bet, and this is
//! the instrument. It is ordinary `std::fs` code with no nife in it, so the same function runs on
//! nife through a granted directory and on a host through a path, which is the only way a
//! comparison runs the same logic on both sides.
//!
//! # What it separates
//!
//! One number for a walk hides which half of the system is expensive, so [`price`] measures four
//! things over [`filesystem_protocol::fixture::walk`]'s tree, each a slope over one variable with
//! the others held still:
//!
//! - **per component**: reading the one small file at each level of a chain of directories, from
//!   one component below the grant to ten. The slope is what one more name in a path costs.
//! - **per entry**: listing a directory of 128 files against listing one of 1, each with its
//!   `file_type`, which is free on nife because `READDIR` carries it. The slope is what one more
//!   entry in a listing costs.
//! - **per byte**: reading a 4 KiB file against a 256 KiB one. The slope is what the search half
//!   of a search tool pays, and it exercises the block path rather than the name path.
//! - **the whole walk**: a recursive `read_dir` and read of every file, in the shape `walkdir` and
//!   `ignore` walk in, with the count of path components it resolved, so the three slopes can be
//!   checked against the total rather than trusted.
//!
//! Each slope's points are the median of [`REPS`] timed runs after one untimed one. The whole walk
//! is timed once, warm, straight after the walk that counted: it is by far the longest figure, and
//! under an emulator in CI five of it cost more wall clock than the rest of the test.
//!
//! # Name
//!
//! Name: provisional 2026-09-26 (milestone 121's lane). A noun for what the crate does to a walk,
//! in the `_pricing` shape nothing else in the tree uses yet; calef names crates.
//!
//! # BUGS
//!
//! - **A median of three is not statistics.** It removes a stray preemption and nothing else. The
//!   figures are magnitudes and directions; a published comparison wants more runs and a spread.
//! - **Warm, not cold.** The untimed run primes whatever caches each side has (the FS server's
//!   metadata cache on nife, the dentry and page caches on a Unix host), so these are the costs of
//!   a second walk. A first walk over a cold disk is a different number and is not taken here.
//! - **The per-entry slope is inside one `READDIR` page.** 128 four-byte names fit in one; a
//!   directory of more than about 680 such names would add a message per page, which this does not
//!   price. See `fixture::walk::WIDE_COUNT`.
//! - **Path-shaped only.** It walks with paths, as `walkdir` and `ignore` do, which on nife means
//!   every open re-resolves every component from the grant. A walker holding `std::fs::Dir` handles
//!   would pay one hop per directory instead, and nothing here measures that shape.

use std::path::Path;
use std::time::Instant;
use std::{fs, io};

use filesystem_protocol::fixture::walk as tree;

/// Timed runs per slope point. Odd, so the median is a run rather than an average of two. Three
/// rather than five because the kernel suite runs this under TCG on three architectures inside a
/// CI job with a time limit; the figures to quote are the totals anyway (notes/walk-pricing.md).
pub const REPS: usize = 3;

/// **Build the priced tree under `root`**, which becomes [`tree::ROOT`]'s contents. Used by the
/// image builder (`xtask`), by this crate's own test, and by a host that wants to price the same
/// shape; nife never calls it, because its copy is on the disk image.
pub fn stage(root: &Path) -> io::Result<()> {
    let mut level = root.join(tree::CHAIN);
    for i in 0..=tree::DEPTH {
        if i > 0 {
            level = level.join(tree::LEVEL);
        }
        fs::create_dir_all(&level)?;
        fs::write(level.join(tree::LEVEL_FILE), tree::SMALL_BODY)?;
    }
    let wide = root.join(tree::WIDE);
    fs::create_dir_all(&wide)?;
    for i in 0..tree::WIDE_COUNT {
        fs::write(wide.join(name(i)), tree::SMALL_BODY)?;
    }
    let narrow = root.join(tree::NARROW);
    fs::create_dir_all(&narrow)?;
    fs::write(narrow.join(name(0)), tree::SMALL_BODY)?;
    let sizes = root.join(tree::SIZES);
    fs::create_dir_all(&sizes)?;
    for (file, len) in tree::SIZE_FILES {
        let body: Vec<u8> = (0..len).map(tree::sized_byte).collect();
        fs::write(sizes.join(file), body)?;
    }
    Ok(())
}

/// [`tree::wide_name`] as a string.
fn name(i: usize) -> String {
    tree::wide_name(i).iter().map(|&b| b as char).collect()
}

/// What one full walk touched. Deterministic for a given tree, which is what makes it assertable
/// where the timings are not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    /// Entries `read_dir` handed back, directories and files alike.
    pub entries: usize,
    /// Files opened and read to the end.
    pub files: usize,
    /// Bytes those reads returned.
    pub bytes: usize,
    /// Path components resolved, summed over every `read_dir` and every open: the number of hops
    /// a component-at-a-time resolver pays, and the number a multi-component one would save.
    pub components: usize,
}

/// **Walk `root` recursively and read every file**, the way `walkdir` does: list, and for each
/// entry either descend by the path the listing handed back or open that path.
///
/// An error anywhere is returned rather than skipped, because a walk that silently finds nothing
/// where it could not look is the failure a search tool must never have. That is the property the
/// refusal half of milestone 121 asserts through this function.
pub fn walk(root: &Path) -> io::Result<Totals> {
    let mut t = Totals::default();
    walk_from(root, 0, &mut t)?;
    Ok(t)
}

fn walk_from(dir: &Path, depth: usize, t: &mut Totals) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        t.entries += 1;
        // The entry's own depth below the root is the component count of the path about to be
        // resolved, for a descent and an open alike.
        t.components += depth + 1;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            walk_from(&path, depth + 1, t)?;
        } else {
            t.bytes += fs::read(&path)?.len();
            t.files += 1;
        }
    }
    Ok(())
}

/// The median of [`REPS`] timed runs of `f`, in nanoseconds, after one untimed run.
fn median_ns(mut f: impl FnMut() -> io::Result<()>) -> io::Result<u64> {
    f()?;
    let mut runs = [0u64; REPS];
    for run in &mut runs {
        let t0 = Instant::now();
        f()?;
        *run = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
    }
    runs.sort_unstable();
    Ok(runs[REPS / 2])
}

/// Least-squares slope of `ys` over `xs`, in the units of `ys` per unit of `xs`.
fn slope(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len() as f64;
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let num: f64 = xs.iter().zip(ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let den: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
    num / den
}

/// Everything [`price`] measured. Timings are nanoseconds as the platform's `Instant` reports
/// them, so under an emulator they are the emulator's time and only their ratios mean anything.
#[derive(Clone, Debug)]
pub struct Price {
    /// What the whole walk touched.
    pub totals: Totals,
    /// Reading the chain's file at each level, shallowest first: `[k]` is `k + 2` components below
    /// the root (`chain`, `k` levels, the file).
    pub by_depth: Vec<u64>,
    /// The fitted cost of one more component, from [`Self::by_depth`].
    pub per_component: f64,
    /// Listing the one-entry directory, with `file_type` on each entry.
    pub list_narrow: u64,
    /// Listing the wide one.
    pub list_wide: u64,
    /// Reading the smallest and largest sized file.
    pub read_small: u64,
    /// See [`Self::read_small`].
    pub read_large: u64,
    /// The whole walk, one warm run.
    pub whole: u64,
}

impl Price {
    /// What one more entry in a listing costs.
    pub fn per_entry(&self) -> f64 {
        (self.list_wide as f64 - self.list_narrow as f64) / (tree::WIDE_COUNT - 1) as f64
    }

    /// What one more KiB read costs.
    pub fn per_kib(&self) -> f64 {
        let (small, large) = (tree::SIZE_FILES[0].1, tree::SIZE_FILES[2].1);
        (self.read_large as f64 - self.read_small as f64) / ((large - small) as f64 / 1024.0)
    }
}

/// **Price a walk over the tree at `root`.** See the crate header for what each figure isolates.
pub fn price(root: &Path) -> io::Result<Price> {
    let totals = walk(root)?;

    let mut by_depth = Vec::with_capacity(tree::DEPTH + 1);
    let mut level = root.join(tree::CHAIN);
    for i in 0..=tree::DEPTH {
        if i > 0 {
            level = level.join(tree::LEVEL);
        }
        let file = level.join(tree::LEVEL_FILE);
        by_depth.push(median_ns(|| fs::read(&file).map(drop))?);
    }
    let xs: Vec<f64> = (0..by_depth.len()).map(|k| k as f64).collect();
    let ys: Vec<f64> = by_depth.iter().map(|&y| y as f64).collect();
    let per_component = slope(&xs, &ys);

    let list = |dir: &Path| {
        let dir = dir.to_path_buf();
        median_ns(move || {
            for e in fs::read_dir(&dir)? {
                e?.file_type()?;
            }
            Ok(())
        })
    };
    let list_narrow = list(&root.join(tree::NARROW))?;
    let list_wide = list(&root.join(tree::WIDE))?;

    let sizes = root.join(tree::SIZES);
    let small = sizes.join(tree::SIZE_FILES[0].0);
    let large = sizes.join(tree::SIZE_FILES[2].0);
    let read_small = median_ns(|| fs::read(&small).map(drop))?;
    let read_large = median_ns(|| fs::read(&large).map(drop))?;

    // Once, warm: the walk that counted above is its untimed run.
    let t0 = Instant::now();
    walk(root)?;
    let whole = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);

    Ok(Price {
        totals,
        by_depth,
        per_component,
        list_narrow,
        list_wide,
        read_small,
        read_large,
        whole,
    })
}

/// **The first line [`report`] prints**, and the only one a gate may assert on: the counts, which
/// are a fact about the tree and the walk and not about the machine.
pub fn totals_line(t: &Totals) -> String {
    format!(
        "walk visited {} entries, {} files, {} bytes, {} components",
        t.entries, t.files, t.bytes, t.components
    )
}

/// The priced figures as the lines a person reads, [`totals_line`] first.
pub fn report(p: &Price) -> Vec<String> {
    vec![
        totals_line(&p.totals),
        format!(
            "walk per component {:.0} ns (depth 2: {} ns, depth {}: {} ns)",
            p.per_component,
            p.by_depth[0],
            tree::DEPTH + 2,
            p.by_depth[tree::DEPTH],
        ),
        format!(
            "walk per entry {:.0} ns (list 1: {} ns, list {}: {} ns)",
            p.per_entry(),
            p.list_narrow,
            tree::WIDE_COUNT,
            p.list_wide,
        ),
        format!(
            "walk per KiB {:.0} ns (read 4 KiB: {} ns, read 256 KiB: {} ns)",
            p.per_kib(),
            p.read_small,
            p.read_large,
        ),
        format!("walk whole {} ns", p.whole),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch directory in the host temp dir, unique per test and process.
    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("walk_pricing-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// **The walker visits exactly the fixture**, and the fixture's own arithmetic agrees. This is
    /// the claim the nife gate asserts from inside a confined process; here it is proven against a
    /// filesystem nobody disputes, so a disagreement there is nife's and not the walker's.
    #[test]
    fn the_walk_visits_what_the_fixture_says_it_holds() {
        let root = scratch("walk");
        stage(&root).expect("stage the tree");
        let t = walk(&root).expect("walk the tree");
        assert_eq!(t.entries, tree::WALK_ENTRIES);
        assert_eq!(t.files, tree::WALK_FILES);
        assert_eq!(t.bytes, tree::WALK_BYTES);
        assert_eq!(t.components, tree::WALK_COMPONENTS);
        let _ = fs::remove_dir_all(&root);
    }

    /// **The report a gate asserts on leads with the counts**, and every figure the crate header
    /// promises is printed. The timings are the host's and are not checked; the shape is, because
    /// the nife gate finds its lines by these prefixes.
    #[test]
    fn a_priced_walk_reports_the_counts_first_and_every_figure() {
        let root = scratch("price");
        stage(&root).expect("stage the tree");
        let p = price(&root).expect("price the walk");
        assert_eq!(p.by_depth.len(), tree::DEPTH + 1);
        assert!(p.whole > 0 && p.list_wide > 0 && p.read_large > 0);
        assert!(p.per_entry().is_finite() && p.per_kib().is_finite());
        let lines = report(&p);
        assert_eq!(lines[0], totals_line(&p.totals));
        for (line, prefix) in lines[1..].iter().zip([
            "walk per component",
            "walk per entry",
            "walk per KiB",
            "walk whole",
        ]) {
            assert!(
                line.starts_with(prefix),
                "{line:?} does not start {prefix:?}"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    /// **A walk that cannot look is an error, never an empty result.** The one property a search
    /// tool must have, checked against the host's own refusal: a missing root is `NotFound`, not a
    /// walk of zero entries.
    #[test]
    fn a_walk_that_cannot_look_fails_rather_than_finding_nothing() {
        let root = scratch("missing").join("not-there");
        let err = walk(&root).expect_err("walking a missing root succeeded");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    /// The slope is a least-squares fit and not an endpoint difference: a straight line comes back
    /// exactly, and one outlier moves it less than it moves the endpoints.
    #[test]
    fn slope_fits_a_line() {
        let xs = [0.0, 1.0, 2.0, 3.0];
        assert!((slope(&xs, &[5.0, 7.0, 9.0, 11.0]) - 2.0).abs() < 1e-9);
    }
}
