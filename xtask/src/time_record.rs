//! **Where a kernel test leg's time went, written down by the program that spent it**
//! (milestone 807 (the kernel suite reports what each test cost), §254 (a gate prints what each
//! item cost)).
//!
//! Until this module, every per-test figure anyone had for this suite was scraped from the
//! timestamps GitHub stamps on log lines, and two scrapes of one suite disagreed by 17%. Now the
//! guest prints its own record (`crates/test_times`), and this module does three things with it
//! for every booted test image:
//!
//! 1. **Times the host side.** The child's stdout is passed through line by line, exactly as it was
//!    when the child inherited it, and each line is stamped as it arrives. From the stamps come four
//!    phases: `build` (spawn to the guest's first line, which is cargo building the image and QEMU
//!    starting), `boot` (to the runner's `running` line), `suite` (to `test result:`) and `exit` (to
//!    the child's exit).
//! 2. **Proves the record is whole.** Every test the transcript names on a `test <path> ... ` line
//!    has a `time` line, every `time` line names such a test, and their count is the `running` line's.
//!    A record with a gap fails the leg.
//! 3. **Proves the guest clock is honest.** The sum of the guest's per-test times must sit inside
//!    [`ClockCheck`]'s band around the host's span for the same tests. On x86_64 the guest's clock is
//!    a TSC calibrated once at boot, which milestone 571 (the x86 boot calibrates the TSC once, and
//!    can be wrong by 4x) found fragile; nothing else in the tree would notice a guest clock that
//!    lies. A sum outside the band fails the leg.
//!
//! Then it appends one row per phase and per test to the run's time record file, a tab-separated
//! table (see [`record_path`]) that CI uploads as an artifact and [`summary`] turns into the step
//! summary (§254, Fork 3).
//!
//! # The file
//!
//! A header line, then rows of five tab-separated fields (shown here with ` | ` for the tabs):
//!
//! ```text
//! leg | image | kind | name | milliseconds
//! riscv64 | system_tests | phase | suite | 129034
//! riscv64 | system_tests | test | system_tests::display_tests::a_bitmap_font | 9912
//! ```
//!
//! `leg` is the architecture plus whatever makes this boot a different machine (`cpu=rva23s64`,
//! `iommu=amd`, `ovmf`, `root-port`). `kind` is `phase` or `test`. Milestone 808 (every gate
//! accounts for its time) adds kinds for the gates beyond the kernel suite; a reader skips a kind it
//! does not know.
//!
//! # EXAMPLES
//!
//! ```text
//! $ cargo xtask test --arch riscv64
//! ...
//! time-record: riscv64 system_tests: 320 tests, guest 127.4 s against host 129.0 s (0.99)
//! $ cargo xtask time-summary target/time-record.tsv >> "$GITHUB_STEP_SUMMARY"
//! ```
//!
//! # BUGS
//!
//! - **Under `--hvf` nothing is recorded.** That leg reads its own transcript to work around a
//!   semihosting exit HVF does not deliver (`hvf_kernel_leg`), and it runs on a laptop, never in CI,
//!   so it was left alone. The guest still prints its block there.
//! - **A filtered run (`--test`) is not recorded either**, apart from x86_64's one-test root-port
//!   boot, which is part of every unfiltered run. A filtered run is a person chasing one test, and
//!   the figure is on their screen.
//! - **The phases are host wall time on a shared runner**, so one run is a premise check and not a
//!   baseline. Milestone 808's daily record takes medians.
//! - **`build` includes QEMU's start-up**, because the first thing the host can see is the guest's
//!   first line. It is under a second against tens of seconds of cargo.
//!
//! Name: provisional (milestone 807's lane, 2026-10-07 UTC), the module, the `time-summary`
//! subcommand, the `NIFE_TIME_RECORD` variable and the file's default name alike. §254 left every
//! name in this design to a separate ratification.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::host::{selected_by, workspace_root};

/// One line of the child's stdout, with how long after the spawn it arrived.
#[derive(Clone, Debug)]
pub(crate) struct Stamped {
    pub(crate) at: Duration,
    pub(crate) text: String,
}

/// The reader that passes a child's stdout through and stamps each line. Spawned beside the child;
/// [`Watch::finish`] joins it once the child has exited.
pub(crate) struct Watch {
    started: Instant,
    reader: JoinHandle<Vec<Stamped>>,
}

impl Watch {
    /// Spawn `command` with its stdout piped through this process's stdout.
    ///
    /// **The bytes are copied the moment they arrive, not a line at a time**, so a test that hangs
    /// after printing `test x ... ` shows that line live, exactly as it did when the child
    /// inherited stdout. Only the stamping waits for a newline.
    pub(crate) fn spawn(command: &mut Command) -> std::io::Result<(Child, Watch)> {
        let started = Instant::now();
        let mut child = command.stdout(Stdio::piped()).spawn()?;
        let mut out = child.stdout.take().expect("stdout was piped");
        let reader = std::thread::spawn(move || {
            let mut lines = Vec::new();
            let mut partial = Vec::new();
            let mut buf = [0u8; 8192];
            let stdout = std::io::stdout();
            loop {
                let n = match out.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                let mut lock = stdout.lock();
                let _ = lock.write_all(&buf[..n]);
                let _ = lock.flush();
                drop(lock);
                let at = started.elapsed();
                for &b in &buf[..n] {
                    if b == b'\n' {
                        lines.push(Stamped {
                            at,
                            text: String::from_utf8_lossy(&partial).into_owned(),
                        });
                        partial.clear();
                    } else {
                        partial.push(b);
                    }
                }
            }
            if !partial.is_empty() {
                lines.push(Stamped {
                    at: started.elapsed(),
                    text: String::from_utf8_lossy(&partial).into_owned(),
                });
            }
            lines
        });
        Ok((child, Watch { started, reader }))
    }

    /// Join the reader once the child has exited, and say when that was.
    pub(crate) fn finish(self) -> (Vec<Stamped>, Duration) {
        let exited = self.started.elapsed();
        let lines = self.reader.join().unwrap_or_default();
        (lines, exited)
    }
}

/// `cargo <args>` for one test image, timed and recorded. The drop-in for `run("cargo", args)` on
/// an unfiltered kernel leg: the verdict is the child's, and then this module's two checks.
pub(crate) fn cargo_test(leg: &str, image: &str, args: &[&str]) -> bool {
    let (mut child, watch) = match Watch::spawn(Command::new("cargo").args(args)) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("failed to run cargo: {e}");
            return false;
        }
    };
    let ok = child.wait().map(|s| s.success()).unwrap_or(false);
    let (lines, exited) = watch.finish();
    account(leg, image, &lines, exited, ok) && ok
}

/// The leg's name for the record: the architecture, then whatever in the environment makes this a
/// different machine from the plain one. Read from the same variables the runners read, so the
/// record cannot name a machine the boot did not have.
pub(crate) fn leg(arch: &str) -> String {
    let mut name = arch.to_string();
    if let Ok(cpu) = std::env::var("NIFE_CPU")
        && !cpu.is_empty()
    {
        name.push_str(" cpu=");
        name.push_str(&cpu);
    }
    if let Ok(iommu) = std::env::var("NIFE_IOMMU")
        && !iommu.is_empty()
    {
        name.push_str(" iommu=");
        name.push_str(&iommu);
    }
    name
}

/// Where the rows go. `NIFE_TIME_RECORD` when it is set, and then the caller owns the file's
/// lifetime and rows are appended (`script/cpu-matrix` runs `cargo xtask test` once per model into
/// one file). Otherwise `target/time-record.tsv`, which [`start`] empties at the top of each
/// `cargo xtask test`.
pub(crate) fn record_path() -> PathBuf {
    match std::env::var_os("NIFE_TIME_RECORD") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => workspace_root().join("target/time-record.tsv"),
    }
}

/// Called once at the top of `cargo xtask test`: a run without `NIFE_TIME_RECORD` starts its own
/// file, so the default file always describes the last run and never a mixture.
pub(crate) fn start() {
    if std::env::var_os("NIFE_TIME_RECORD").is_none_or(|p| p.is_empty()) {
        let _ = std::fs::remove_file(record_path());
    }
}

/// The header line of the record file. A reader checks it before trusting the columns.
pub(crate) const HEADER: &str = "leg\timage\tkind\tname\tmilliseconds";

/// One row of the record file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    pub(crate) leg: String,
    pub(crate) image: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) milliseconds: u64,
}

impl Row {
    fn tsv(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}",
            self.leg, self.image, self.kind, self.name, self.milliseconds
        )
    }

    fn parse(line: &str) -> Option<Row> {
        let mut f = line.split('\t');
        let row = Row {
            leg: f.next()?.to_string(),
            image: f.next()?.to_string(),
            kind: f.next()?.to_string(),
            name: f.next()?.to_string(),
            milliseconds: f.next()?.trim_end().parse().ok()?,
        };
        f.next().is_none().then_some(row)
    }
}

fn append(rows: &[Row]) {
    let path = record_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let fresh = std::fs::metadata(&path).map_or(true, |m| m.len() == 0);
    let mut file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("time-record: cannot open {}: {e}", path.display());
            return;
        }
    };
    let mut text = String::new();
    if fresh {
        text.push_str(HEADER);
        text.push('\n');
    }
    for row in rows {
        text.push_str(&row.tsv());
        text.push('\n');
    }
    if let Err(e) = file.write_all(text.as_bytes()) {
        eprintln!("time-record: cannot write {}: {e}", path.display());
    }
}

/// What one image's transcript says, read once and checked.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Reading {
    /// The `running` line's count of selected tests.
    pub(crate) selected: Option<usize>,
    /// Stamps of the four markers the phases are cut at.
    pub(crate) first_line: Option<Duration>,
    pub(crate) running: Option<Duration>,
    pub(crate) heading: Option<Duration>,
    pub(crate) result: Option<Duration>,
    /// Test paths from `test <path> ... ` lines, in order.
    pub(crate) tests: Vec<String>,
    /// The guest's record, in order.
    pub(crate) record: Vec<(String, u64)>,
    /// Lines inside the block that did not parse, verbatim.
    pub(crate) malformed: Vec<String>,
}

/// A transcript line that announces a test: `test <path> ... `, where the path is one `type_name`
/// (no spaces, at least one `::`). A test's own output that begins with "test " does not match
/// unless it also looks exactly like this.
fn announced_test(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("test ")?;
    let (path, _) = rest.split_once(" ... ")?;
    (path.contains("::") && !path.contains(char::is_whitespace)).then_some(path)
}

/// Read a stamped transcript into a [`Reading`].
pub(crate) fn read(lines: &[Stamped]) -> Reading {
    let mut r = Reading {
        first_line: lines.first().map(|l| l.at),
        ..Reading::default()
    };
    let mut in_block = false;
    for line in lines {
        let text = line.text.trim_end_matches('\r');
        if in_block {
            if text.is_empty() {
                in_block = false;
                continue;
            }
            match test_times::parse(text) {
                Ok(l) => r.record.push((l.path.to_string(), l.milliseconds)),
                Err(_) => r.malformed.push(text.to_string()),
            }
            continue;
        }
        if text == test_times::HEADING {
            in_block = true;
            r.heading = Some(line.at);
        } else if text.starts_with("test result:") {
            r.result = Some(line.at);
        } else if let Some(path) = announced_test(text) {
            r.tests.push(path.to_string());
        } else if r.running.is_none()
            && let Some(n) = running_count(text)
        {
            r.selected = Some(n);
            r.running = Some(line.at);
        }
    }
    r
}

/// `N` from the runner's `running N tests`, or `K` from `running K of N tests (filter: ...)`.
fn running_count(line: &str) -> Option<usize> {
    if let Some(k) = selected_by(line) {
        return Some(k);
    }
    let rest = line.trim_start().strip_prefix("running ")?;
    let (count, rest) = rest.split_once(' ')?;
    (rest == "tests" || rest == "test")
        .then(|| count.parse().ok())
        .flatten()
}

/// Whether the record names exactly the tests the transcript ran. `Err` says what is missing or
/// extra, first few of each.
pub(crate) fn whole(r: &Reading) -> Result<(), String> {
    use std::collections::BTreeSet;
    // A filtered run that selected nothing in this image (`script/test --test` names a test that
    // lives in the other image) has no test to time, and the kernel prints no record for it. The
    // QEMU legs already count such a leg; the OVMF leg failed it for want of a record until
    // milestone 353 (the aarch64 half of 74) hit it on 2026-10-07 (UTC) with a test
    // that lives only in the system-tests image.
    if r.heading.is_none() && r.selected == Some(0) && r.tests.is_empty() {
        return Ok(());
    }
    if r.heading.is_none() {
        return Err("the image printed no per-test time record".into());
    }
    if !r.malformed.is_empty() {
        return Err(format!(
            "{} record line(s) did not parse, the first {:?}",
            r.malformed.len(),
            r.malformed[0]
        ));
    }
    let ran: BTreeSet<&str> = r.tests.iter().map(String::as_str).collect();
    let timed: BTreeSet<&str> = r.record.iter().map(|(p, _)| p.as_str()).collect();
    let untimed: Vec<&&str> = ran.difference(&timed).take(3).collect();
    let unran: Vec<&&str> = timed.difference(&ran).take(3).collect();
    if !untimed.is_empty() || !unran.is_empty() {
        return Err(format!(
            "the record and the transcript disagree: ran but not timed {untimed:?}, timed but not \
             announced {unran:?}"
        ));
    }
    if let Some(n) = r.selected
        && n != r.record.len()
    {
        return Err(format!(
            "the runner selected {n} tests and the record has {} lines",
            r.record.len()
        ));
    }
    Ok(())
}

/// The band the guest's sum must sit in, around the host's span for the same tests.
///
/// The host span runs from the `running` line to the record's heading, so it holds every test plus
/// what the guest does between them (the name, the frame ledger's charge) and the reports after the
/// last one. The guest's sum holds only the bodies. So the sum is below the span by an overhead,
/// and it can exceed the span only if the guest's clock runs fast.
///
/// - **Upper: the sum may not exceed the span by more than 5% plus half a second.** Under TCG the
///   guest's counter follows the host's clock and stops when the VM stops, so it cannot honestly
///   run ahead. The slack is for the host's stamp landing a pipe read late.
/// - **Lower: the sum must reach half the span, less three seconds.** The overhead between tests is
///   console output and a ledger read; half is far below any measured share and far above the 25%
///   a 4x TSC error would give. The absolute slack keeps a short image (137 kernel unit tests in
///   about six seconds on riscv64) from failing on fixed costs.
///
/// Both bounds are first guesses stated as such. The first runs are the measurement, every run
/// prints its ratio, and a band that turns out wrong is widened here with the run that showed it.
pub(crate) struct ClockCheck;

impl ClockCheck {
    const UPPER_RATIO: f64 = 1.05;
    const UPPER_SLACK_MS: f64 = 500.0;
    const LOWER_RATIO: f64 = 0.5;
    const LOWER_SLACK_MS: f64 = 3000.0;

    /// `Ok` when `guest_ms` is honest against `host_ms`, otherwise the sentence that says why not.
    pub(crate) fn judge(guest_ms: u64, host_ms: u64) -> Result<(), String> {
        let (g, h) = (guest_ms as f64, host_ms as f64);
        let high = h * Self::UPPER_RATIO + Self::UPPER_SLACK_MS;
        let low = h * Self::LOWER_RATIO - Self::LOWER_SLACK_MS;
        if g > high {
            return Err(format!(
                "the guest says its tests took {guest_ms} ms but the host saw the suite take \
                 {host_ms} ms: the guest clock runs fast (the bound is {high:.0} ms)"
            ));
        }
        if g < low {
            return Err(format!(
                "the guest says its tests took {guest_ms} ms but the host saw the suite take \
                 {host_ms} ms: the guest clock runs slow (the bound is {low:.0} ms)"
            ));
        }
        Ok(())
    }
}

fn ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// The phases a reading supports, as rows. A phase whose end marker never came is left out rather
/// than written as a guess, so a failed boot records its `build` and nothing after.
fn phases(leg: &str, image: &str, r: &Reading, exited: Duration) -> Vec<Row> {
    let row = |name: &str, span: Duration| Row {
        leg: leg.to_string(),
        image: image.to_string(),
        kind: "phase".into(),
        name: name.into(),
        milliseconds: ms(span),
    };
    let mut rows = vec![row("build", r.first_line.unwrap_or(exited))];
    if let (Some(first), Some(running)) = (r.first_line, r.running) {
        rows.push(row("boot", running.saturating_sub(first)));
        if let Some(result) = r.result {
            rows.push(row("suite", result.saturating_sub(running)));
            rows.push(row("exit", exited.saturating_sub(result)));
        }
    }
    rows
}

/// Read, check and record one image's transcript. Returns whether both checks held; a child that
/// failed is recorded as far as it got and judged by its own status, not by these checks.
pub(crate) fn account(
    leg: &str,
    image: &str,
    lines: &[Stamped],
    exited: Duration,
    child_ok: bool,
) -> bool {
    let r = read(lines);
    let mut rows = phases(leg, image, &r, exited);
    rows.extend(r.record.iter().map(|(path, ms)| Row {
        leg: leg.to_string(),
        image: image.to_string(),
        kind: "test".into(),
        name: path.clone(),
        milliseconds: *ms,
    }));
    append(&rows);
    if !child_ok {
        return true;
    }
    if let Err(why) = whole(&r) {
        eprintln!("time-record: {leg} {image}: {why}");
        return false;
    }
    let guest: u64 = r.record.iter().map(|(_, ms)| ms).sum();
    let host = match (r.running, r.heading) {
        (Some(a), Some(b)) => ms(b.saturating_sub(a)),
        _ => 0,
    };
    let ratio = if host == 0 {
        0.0
    } else {
        guest as f64 / host as f64
    };
    eprintln!(
        "time-record: {leg} {image}: {} tests, guest {:.1} s against host {:.1} s ({ratio:.2})",
        r.record.len(),
        guest as f64 / 1000.0,
        host as f64 / 1000.0,
    );
    if let Err(why) = ClockCheck::judge(guest, host) {
        eprintln!("time-record: {leg} {image}: {why}");
        eprintln!(
            "    The bounds are `ClockCheck` in xtask/src/time_record.rs. On x86_64 suspect the TSC \
             calibration first (milestone 571)."
        );
        return false;
    }
    true
}

/// `cargo xtask time-summary [file...]`: the record as markdown for a CI step summary (§254, Fork
/// 3). Per leg and image, the four phases; then the slowest tests and the slowest modules over the
/// whole file. Defaults to [`record_path`]. A missing file prints a line saying so and succeeds,
/// because the summary step runs `if: always()` and a job that failed before its first boot has
/// nothing to summarize.
pub(crate) fn summary() -> bool {
    let mut paths: Vec<PathBuf> = std::env::args().skip(2).map(PathBuf::from).collect();
    if paths.is_empty() {
        paths.push(record_path());
    }
    let mut rows = Vec::new();
    for path in &paths {
        match std::fs::read_to_string(path) {
            Ok(text) => match parse_file(&text) {
                Ok(mut r) => rows.append(&mut r),
                Err(why) => {
                    eprintln!("time-summary: {}: {why}", path.display());
                    return false;
                }
            },
            Err(_) => println!("No time record at `{}`.", path.display()),
        }
    }
    print!("{}", markdown(&rows));
    true
}

/// Read a record file. `Err` when the header is not [`HEADER`] or a row does not parse.
pub(crate) fn parse_file(text: &str) -> Result<Vec<Row>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("the first line is not the time record's header".into());
    }
    lines
        .enumerate()
        .filter(|(_, l)| !l.is_empty())
        .map(|(i, l)| Row::parse(l).ok_or(format!("row {} does not parse: {l:?}", i + 2)))
        .collect()
}

/// How many tests and modules the summary lists. Enough to name a drift's leaders; the artifact
/// has the rest.
const SLOWEST: usize = 15;

fn seconds(ms: u64) -> String {
    format!("{:.1}", ms as f64 / 1000.0)
}

/// The step summary's markdown for `rows`.
pub(crate) fn markdown(rows: &[Row]) -> String {
    let mut out = String::new();
    if rows.is_empty() {
        return out;
    }
    out.push_str("### Where the kernel suite's time went\n\n");
    out.push_str("Seconds, host wall time for phases and guest time for tests. The full record is this run's `time-record` artifact (milestone 807).\n\n");

    // Phases, one row per leg and image, in the order they ran.
    let mut order: Vec<(String, String)> = Vec::new();
    let mut phase: BTreeMap<(String, String, String), u64> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.kind == "phase") {
        let key = (r.leg.clone(), r.image.clone());
        if !order.contains(&key) {
            order.push(key);
        }
        *phase
            .entry((r.leg.clone(), r.image.clone(), r.name.clone()))
            .or_default() += r.milliseconds;
    }
    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.kind == "test") {
        *counts.entry((r.leg.clone(), r.image.clone())).or_default() += 1;
    }
    out.push_str("| leg | image | tests | build | boot | suite | exit |\n|---|---|---:|---:|---:|---:|---:|\n");
    for (leg, image) in &order {
        let get = |n: &str| {
            phase
                .get(&(leg.clone(), image.clone(), n.to_string()))
                .map_or("".to_string(), |&v| seconds(v))
        };
        out.push_str(&format!(
            "| {leg} | {image} | {} | {} | {} | {} | {} |\n",
            counts
                .get(&(leg.clone(), image.clone()))
                .copied()
                .unwrap_or(0),
            get("build"),
            get("boot"),
            get("suite"),
            get("exit"),
        ));
    }

    let mut tests: Vec<&Row> = rows.iter().filter(|r| r.kind == "test").collect();
    tests.sort_by_key(|r| std::cmp::Reverse(r.milliseconds));
    out.push_str("\n#### Slowest tests\n\n| seconds | leg | test |\n|---:|---|---|\n");
    for r in tests.iter().take(SLOWEST) {
        out.push_str(&format!(
            "| {} | {} | `{}` |\n",
            seconds(r.milliseconds),
            r.leg,
            r.name
        ));
    }

    let mut modules: BTreeMap<(String, String), (u64, usize)> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.kind == "test") {
        let module = r.name.rsplit_once("::").map_or(r.name.as_str(), |(m, _)| m);
        let e = modules
            .entry((r.leg.clone(), module.to_string()))
            .or_default();
        e.0 += r.milliseconds;
        e.1 += 1;
    }
    let mut modules: Vec<_> = modules.into_iter().collect();
    modules.sort_by_key(|m| std::cmp::Reverse(m.1.0));
    out.push_str(
        "\n#### Slowest modules\n\n| seconds | tests | leg | module |\n|---:|---:|---|---|\n",
    );
    for ((leg, module), (total, n)) in modules.iter().take(SLOWEST) {
        out.push_str(&format!(
            "| {} | {n} | {leg} | `{module}` |\n",
            seconds(*total)
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64, text: &str) -> Stamped {
        Stamped {
            at: Duration::from_millis(ms),
            text: text.to_string(),
        }
    }

    fn transcript(times: &[(&str, u64)], host_span_ms: u64) -> Vec<Stamped> {
        let mut v = vec![
            at(2000, "nife on riscv64"),
            at(3000, ""),
            at(3000, &format!("running {} tests", times.len())),
        ];
        for (name, _) in times {
            v.push(at(3100, &format!("test {name} ... ok")));
        }
        v.push(at(3000 + host_span_ms, test_times::HEADING));
        for (name, ms) in times {
            v.push(at(
                3000 + host_span_ms,
                &test_times::Line {
                    milliseconds: *ms,
                    path: name,
                }
                .to_string(),
            ));
        }
        v.push(at(3000 + host_span_ms, ""));
        v.push(at(3001 + host_span_ms, "test result: ok. 2 passed"));
        v
    }

    #[test]
    fn a_whole_record_reads_and_passes_both_checks() {
        let t = transcript(
            &[("kernel::a::one", 4000), ("kernel::a::two", 5000)],
            10_000,
        );
        let r = read(&t);
        assert_eq!(r.selected, Some(2));
        assert_eq!(r.record.len(), 2);
        assert_eq!(whole(&r), Ok(()));
        assert_eq!(ClockCheck::judge(9000, 10_000), Ok(()));
        let p = phases("riscv64", "kernel", &r, Duration::from_millis(14_000));
        let names: Vec<_> = p
            .iter()
            .map(|r| (r.name.as_str(), r.milliseconds))
            .collect();
        assert_eq!(
            names,
            [
                ("build", 2000),
                ("boot", 1000),
                ("suite", 10_001),
                ("exit", 999)
            ]
        );
    }

    #[test]
    fn a_test_the_record_left_out_fails_the_gap_check() {
        let mut t = transcript(&[("kernel::a::one", 4), ("kernel::a::two", 5)], 100);
        t.retain(|l| l.text != "time 5 kernel::a::two");
        assert!(whole(&read(&t)).unwrap_err().contains("kernel::a::two"));
    }

    #[test]
    fn a_missing_block_and_a_malformed_line_both_fail() {
        let mut t = transcript(&[("kernel::a::one", 4)], 100);
        t.retain(|l| !l.text.starts_with("time"));
        assert!(
            whole(&read(&t))
                .unwrap_err()
                .contains("no per-test time record")
        );
        let mut t = transcript(&[("kernel::a::one", 4)], 100);
        for l in &mut t {
            if l.text == "time 4 kernel::a::one" {
                l.text = "time 4 kernel::a::one trailing".into();
            }
        }
        assert!(whole(&read(&t)).unwrap_err().contains("did not parse"));
    }

    #[test]
    fn a_filter_that_selected_nothing_needs_no_record() {
        let t = vec![
            at(2000, "nife on x86_64"),
            at(
                3000,
                "running 0 of 138 tests (filter: some_other_images_test)",
            ),
            at(3001, "test result: ok. 0 passed"),
        ];
        let r = read(&t);
        assert_eq!(r.selected, Some(0));
        assert_eq!(whole(&r), Ok(()));
    }

    #[test]
    fn a_test_line_with_its_own_output_still_names_the_test() {
        assert_eq!(
            announced_test("test kernel::x::y ... [12 s] ok"),
            Some("kernel::x::y")
        );
        assert_eq!(announced_test("test result: ok. 3 passed"), None);
        assert_eq!(announced_test("test the frobnicator ... ok"), None);
    }

    /// Exit criterion 3's host test: the cross-check fails when the two clocks disagree, in both
    /// directions, and holds when they agree.
    #[test]
    fn the_clock_check_fails_when_the_guest_and_host_disagree() {
        // Honest: the bodies are most of the span.
        assert!(ClockCheck::judge(120_000, 129_000).is_ok());
        // A short image's fixed costs are inside the absolute slack.
        assert!(ClockCheck::judge(1_000, 6_400).is_ok());
        // A TSC calibrated 4x fast or slow, milestone 571's failure.
        assert!(
            ClockCheck::judge(4 * 60_000, 60_000)
                .unwrap_err()
                .contains("fast")
        );
        assert!(
            ClockCheck::judge(60_000 / 4, 60_000)
                .unwrap_err()
                .contains("slow")
        );
        // A clock that never calibrated reads zero for every test.
        assert!(ClockCheck::judge(0, 60_000).is_err());
    }

    #[test]
    fn the_record_file_round_trips_and_summarizes() {
        let rows = vec![
            Row {
                leg: "riscv64".into(),
                image: "system_tests".into(),
                kind: "phase".into(),
                name: "suite".into(),
                milliseconds: 129_000,
            },
            Row {
                leg: "riscv64".into(),
                image: "system_tests".into(),
                kind: "test".into(),
                name: "system_tests::display_tests::a_bitmap_font".into(),
                milliseconds: 9_900,
            },
        ];
        let mut text = String::from(HEADER);
        text.push('\n');
        for r in &rows {
            text.push_str(&r.tsv());
            text.push('\n');
        }
        assert_eq!(parse_file(&text), Ok(rows.clone()));
        assert!(parse_file("not a header\n").is_err());
        let md = markdown(&rows);
        assert!(md.contains("| riscv64 | system_tests | 1 |"));
        assert!(md.contains("`system_tests::display_tests`"));
        assert!(md.contains("| 9.9 |"));
    }
}
