//! The scanout check (milestone 29): prove the pixels reached the DEVICE, not only our buffer.
//!
//! The in-guest test proves the framebuffer byte for byte, and cannot do better: the suite runs
//! `-display none` and nothing inside the guest can read QEMU's host-side surface back, so a wrong
//! pixel format or scanout rectangle would pass it and show garbage on a real screen.
//!
//! QEMU's monitor closes that gap, and it works headlessly: `screendump FILE` writes a PPM of the
//! scanout even with no display backend. So the runners take a monitor socket (`NIFE_GPU_MON`), and
//! this drives it **while the ordinary test run is happening**, rather than paying for a second boot:
//! the suite is minutes long per ISA and the pattern stays on the scanout from the display test until
//! QEMU exits, so there is no need to synchronize with the guest at all. Poll, dump, compare; the
//! first match ends the polling.
//!
//! Fail-safe by construction. If the pattern never reaches the scanout, or the display test stops
//! running, or the confinement test's device reset moves after it and wipes the surface, no dump
//! matches and this reports it. Nothing here can make a broken scanout look fine.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::host::workspace_root;
use crate::inbound::InboundProber;

/// The unix socket the QEMU monitor listens on for `arch`. **In /tmp on purpose**: a unix socket path
/// must fit in 104 bytes, and a worktree checkout plus `target/` gets close enough to that limit to
/// break on someone else's machine. The PPM it dumps goes under `target/`, where path length is free.
pub(crate) fn gpu_mon_socket(arch: &str) -> String {
    format!("/tmp/nife-gpu-{arch}-{}.sock", std::process::id())
}

fn gpu_shot_path(arch: &str) -> PathBuf {
    workspace_root().join(format!("target/gpu-scanout-{arch}.ppm"))
}

/// Where the composed screen's matching dump is kept (milestone 33). A separate file because the
/// composed screen is transient: the poll loop overwrites [`gpu_shot_path`] on every dump, so the one
/// that matched has to be copied aside or there is nothing left to look at after a run.
fn gpu_compose_path(arch: &str) -> PathBuf {
    workspace_root().join(format!("target/gpu-compose-{arch}.ppm"))
}

/// Where the display terminal's matching dump is kept (milestone 29's text increment). Transient for
/// the same reason the composed screen is: rung one's pattern replaces it on the same scanout.
fn gpu_text_path(arch: &str) -> PathBuf {
    workspace_root().join(format!("target/gpu-text-{arch}.ppm"))
}

/// Does this PPM hold the pattern rung one's client painted (milestone 29)?
///
/// Compares against `graphics_protocol::pixel`, the same definition the client painted from and the kernel
/// test digested against, so the host cannot disagree with the guest about what the pattern is.
fn scanout_holds_the_pattern(ppm: &[u8]) -> Result<(), String> {
    scanout_matches(ppm, graphics_protocol::pixel)
}

/// Does this PPM hold the screen rung two's compositor composed (milestone 33)?
///
/// The same check against a different definition: `compositor::expected_screen_pixel` with every window
/// of the scene committed, which is the picture the kernel test predicted and the capture client
/// digested. **This is the check a guest-side digest cannot replace.** Three witnesses inside the
/// guest agree about the framebuffer; only the host can see what the device is actually scanning out,
/// so a wrong pixel format, a wrong scanout rectangle, or a compositor that wrote its picture
/// somewhere other than the scanout would pass all three and fail here.
fn scanout_holds_the_composed_screen(ppm: &[u8]) -> Result<(), String> {
    scanout_matches(ppm, |x, y| {
        compositor::expected_screen_pixel(compositor::SCENE.len(), x, y)
    })
}

/// Does this PPM hold the **text** the display terminal drew (milestone 29's remaining increment)?
///
/// The definition is the VT engine itself, run here on the host over `video_terminal::script`, the same script
/// the kernel sent the terminal and the same engine the terminal drew from. So this is not "is there
/// ink on the screen": it is every pixel of every glyph, in the right cell, in the right colour,
/// with the cursor where the engine says it is.
///
/// **This is the check a guest-side digest cannot replace**, and for text it matters more than for a
/// pattern: a wrong pixel format turns a test pattern into an odd-looking test pattern, and it turns
/// text into text nobody can read. Its negative control is
/// `tests::the_scanout_check_rejects_text_that_is_one_letter_wrong`.
fn scanout_holds_the_terminals_text(ppm: &[u8]) -> Result<(), String> {
    // `Vt::new` then the script fed into it rather than the old `-> Vt` shape: a `Vt` is
    // hundreds of KiB since milestone 142's grid growth, and while this host binary's stack has
    // room either way, the crate's own signature changed for its kernel-side callers and this is
    // the one shape that works for both (see `Vt`'s and `script::full_screen`'s own doc comments).
    //
    // **The scrolled script, since the paint path (2026-09-30)**: the kernel-side test scrolls
    // the terminal before holding the picture up, and this check grades the same end state, so
    // what the host verifies includes the scroll path the terminal now takes (pixels moved
    // rather than re-rendered), not just a static page.
    let mut expect =
        video_terminal::Vt::new(video_terminal::script::COLS, video_terminal::script::ROWS);
    video_terminal::script::scrolled_full_screen(&mut expect, None);
    scanout_matches(ppm, |x, y| expect.pixel(x, y))
}

/// Compare a `screendump` PPM against a per-pixel definition of what should be on the screen.
///
/// The geometry must match too: a scanout of the wrong size is a `SET_SCANOUT` bug, not a near miss.
///
/// Returns `Err(reason)` rather than a bool so a mismatch says which pixel and what it should have
/// been, since "the screen is wrong" is otherwise the least actionable failure in graphics.
fn scanout_matches(ppm: &[u8], want_pixel: impl Fn(u32, u32) -> u32) -> Result<(), String> {
    // P6 header: "P6\n<w> <h>\n<maxval>\n", then w*h*3 bytes, RGB per pixel.
    let text = String::from_utf8_lossy(&ppm[..ppm.len().min(64)]).to_string();
    let mut fields = text.split_ascii_whitespace();
    if fields.next() != Some("P6") {
        return Err("not a P6 PPM".into());
    }
    let w: u32 = fields
        .next()
        .and_then(|f| f.parse().ok())
        .ok_or("no width")?;
    let h: u32 = fields
        .next()
        .and_then(|f| f.parse().ok())
        .ok_or("no height")?;
    let maxval = fields.next().ok_or("no maxval")?;
    if maxval != "255" {
        return Err(format!("maxval {maxval}, expected 255"));
    }
    if (w, h) != (graphics_protocol::WIDTH, graphics_protocol::HEIGHT) {
        return Err(format!(
            "scanout is {w}x{h}, the surface is {}x{}",
            graphics_protocol::WIDTH,
            graphics_protocol::HEIGHT
        ));
    }
    // The pixel data starts after the fourth whitespace-terminated field. Find it by walking the
    // header rather than assuming a byte offset, because QEMU is free to format the header its way.
    let mut seen = 0;
    let mut i = 0;
    while i < ppm.len() && seen < 4 {
        if ppm[i].is_ascii_whitespace() {
            seen += 1;
            while seen < 4 && i + 1 < ppm.len() && ppm[i + 1].is_ascii_whitespace() {
                i += 1;
            }
        }
        i += 1;
    }
    let pixels = &ppm[i..];
    let want_len = (w * h * 3) as usize;
    if pixels.len() < want_len {
        // A dump caught mid-write. Not a failure, just not usable yet.
        return Err(format!(
            "short by {} bytes (QEMU may still be writing)",
            want_len - pixels.len()
        ));
    }
    for y in 0..h {
        for x in 0..w {
            let o = ((y * w + x) * 3) as usize;
            let want = want_pixel(x, y);
            let (r, g, b) = (
                ((want >> 16) & 0xff) as u8,
                ((want >> 8) & 0xff) as u8,
                (want & 0xff) as u8,
            );
            if (pixels[o], pixels[o + 1], pixels[o + 2]) != (r, g, b) {
                return Err(format!(
                    "pixel ({x},{y}) is rgb({},{},{}), it should be rgb({r},{g},{b})",
                    pixels[o],
                    pixels[o + 1],
                    pixels[o + 2],
                ));
            }
        }
    }
    Ok(())
}

/// Parse a `screendump` P6 PPM into `(width, height, rgb bytes)`. [`scanout_matches`]'s own header
/// walk, lifted out for milestone 177 (wire the graphical terminal stack into the real interactive
/// boot) and its graphical swish-check leg, which reads a screendump's text
/// back out instead of comparing it against a picture computed in advance (there is no such picture
/// for a live, typed shell session; see [`decode_cell`]).
fn parse_ppm(ppm: &[u8]) -> Result<(u32, u32, &[u8]), String> {
    let text = String::from_utf8_lossy(&ppm[..ppm.len().min(64)]).to_string();
    let mut fields = text.split_ascii_whitespace();
    if fields.next() != Some("P6") {
        return Err("not a P6 PPM".into());
    }
    let w: u32 = fields
        .next()
        .and_then(|f| f.parse().ok())
        .ok_or("no width")?;
    let h: u32 = fields
        .next()
        .and_then(|f| f.parse().ok())
        .ok_or("no height")?;
    let maxval = fields.next().ok_or("no maxval")?;
    if maxval != "255" {
        return Err(format!("maxval {maxval}, expected 255"));
    }
    let mut seen = 0;
    let mut i = 0;
    while i < ppm.len() && seen < 4 {
        if ppm[i].is_ascii_whitespace() {
            seen += 1;
            while seen < 4 && i + 1 < ppm.len() && ppm[i + 1].is_ascii_whitespace() {
                i += 1;
            }
        }
        i += 1;
    }
    let pixels = &ppm[i..];
    let want_len = (w * h * 3) as usize;
    if pixels.len() < want_len {
        return Err(format!(
            "short by {} bytes (QEMU may still be writing)",
            want_len - pixels.len()
        ));
    }
    Ok((w, h, &pixels[..want_len]))
}

/// **Read one glyph cell back out of a screendump**, the reverse of the direction every other
/// scanout check in this file runs: those compare against a picture predicted in advance, and there
/// is no way to predict a live, typed shell session's screen in advance (milestone 177's graphical
/// swish-check leg: the boot banner's exact wrapped, scrolled position in an 18x8 grid depends on
/// wording nobody wants two copies of, so this reads the picture instead of guessing it).
///
/// Tries every byte in `alphabet` against [`bitmap_font::cell_pixel`]'s own definition, at the
/// terminal's own default colours (`video_terminal::Attr::DEFAULT`; nothing this leg looks for is
/// ever printed with a colour escape). Returns the one that matches every one of the cell's
/// `GLYPH_W * GLYPH_H` pixels exactly, or `None` if nothing in `alphabet` does (a blank cell, a
/// byte outside `alphabet`, or the reversed cursor cell, which this deliberately does not decode:
/// see the module note on why a caller never needs to).
fn decode_cell(w: u32, pixels: &[u8], col: u32, row: u32, alphabet: &[u8]) -> Option<u8> {
    let (fg, bg) = video_terminal::Attr::DEFAULT.colours();
    'byte: for &b in alphabet {
        for gy in 0..bitmap_font::GLYPH_H {
            for gx in 0..bitmap_font::GLYPH_W {
                let (x, y) = (
                    col * bitmap_font::GLYPH_W + gx,
                    row * bitmap_font::GLYPH_H + gy,
                );
                let o = ((y * w + x) * 3) as usize;
                let want = bitmap_font::cell_pixel(b as char, gx, gy, fg, bg);
                let (r, g, bl) = (
                    ((want >> 16) & 0xff) as u8,
                    ((want >> 8) & 0xff) as u8,
                    (want & 0xff) as u8,
                );
                if (pixels[o], pixels[o + 1], pixels[o + 2]) != (r, g, bl) {
                    continue 'byte;
                }
            }
        }
        return Some(b);
    }
    None
}

/// **Read every row of a screendump back into text**, decoding each of the 18x8 grid's cells
/// against `alphabet` and leaving `b'?'` where nothing in it matches. One string per row, so a
/// caller can search for a substring without caring which row it landed on (the boot banner's exact
/// scroll position is exactly what this leg does not want to have to predict).
pub(crate) fn scanout_rows(ppm: &[u8], alphabet: &[u8]) -> Result<Vec<String>, String> {
    let (w, h, pixels) = parse_ppm(ppm)?;
    if (w, h) != (graphics_protocol::WIDTH, graphics_protocol::HEIGHT) {
        return Err(format!(
            "scanout is {w}x{h}, the surface is {}x{}",
            graphics_protocol::WIDTH,
            graphics_protocol::HEIGHT
        ));
    }
    let (cols, rows) = (w / bitmap_font::GLYPH_W, h / bitmap_font::GLYPH_H);
    Ok((0..rows)
        .map(|row| {
            (0..cols)
                .map(|col| decode_cell(w, pixels, col, row, alphabet).unwrap_or(b'?') as char)
                .collect::<String>()
        })
        .collect())
}

/// **Press a key on the guest's keyboard**, over the same monitor the scanout check uses.
///
/// Nothing inside the guest can press a key, which is the point of testing a real input device, so
/// this is the one place the host is an *actor* rather than an observer. `sendkey` sends a press and
/// a release, which is also what makes the driver's handling of the event's value field checkable:
/// counting the release would double every character.
///
/// Sent on every poll, from the start of the run. That needs no synchronization with the guest
/// because QEMU **drops key events until a driver sets `DRIVER_OK`**, so keys pressed before the
/// keyboard driver exists go nowhere, and once it exists the next one lands. `video_terminal::script::HOST_KEY`
/// is the one definition of which key, shared with the kernel test that asserts the byte.
pub(crate) fn sendkey(sock: &str, key: &str) {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    let Ok(mut s) = UnixStream::connect(sock) else {
        return;
    };
    let _ = s.write_all(format!("sendkey {key}\n").as_bytes());
    let _ = s.flush();
}

/// Ask the QEMU monitor on `sock` for a screendump into `out`. Returns false while the socket is not
/// there yet (QEMU still starting, or already gone), which the caller treats as "try again".
pub(crate) fn screendump(sock: &str, out: &Path) -> bool {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    let Ok(mut s) = UnixStream::connect(sock) else {
        return false;
    };
    // The monitor greets us, then takes one command per line. We never read the reply: the evidence is
    // the file, and a reply we misparsed would only be a second way to be wrong.
    let _ = s.write_all(format!("screendump {}\n", out.display()).as_bytes());
    let _ = s.flush();
    // Give QEMU a moment to write the file before the caller reads it. The size check in
    // `scanout_holds_the_pattern` catches a partial write anyway, so this only reduces retries.
    std::thread::sleep(std::time::Duration::from_millis(150));
    true
}

/// **The host's load average across one emulated leg**, so a red timing assertion says whether the
/// machine was busy (milestone 117's third stranger run, 2026-08-18).
///
/// # Why the harness and not the guest
///
/// The whole family of assertions in notes/load-sensitive-assertions.md has one signature: a claim
/// whose truth depends on the host, written from inside a guest that cannot see the host. The guest
/// can measure how late it was; it cannot know whether eleven other QEMUs were on the same eight
/// cores. **This process can**, and it is the only participant that can, which is why the number is
/// printed here rather than woven into a panic message.
///
/// The run that asked for it: `script/test` went red in 2 of 13 aarch64 legs while other lanes had
/// this laptop at a one-minute load average of 45 to 63, and nothing in the transcript said so, so
/// an hour went into a defect that was not there. One line at the failure would have decided it.
///
/// # What it does and does not claim
///
/// It **suggests**, and it says so in its own output. A loaded host does not make a failure
/// spurious; a quiet host does not make it real. What it removes is the reader having to guess,
/// and the peak matters as much as the number at the end: a suite runs for minutes and the
/// one-minute average decays, so a burst of contention halfway through is invisible by the time
/// the leg fails.
///
/// min/mean/peak is `script/repeat-under-load`'s vocabulary, deliberately: the acceptance harness
/// already reports contention in those three numbers, and a reader who has seen one table should
/// recognise this line without learning a second shape.
///
/// # BUGS
///
/// Sampled only while a leg is *running*. A host that was quiet during the leg and thrashing during
/// the build before it produces an honest, unhelpful line. The subprocess is one `uptime` every
/// five seconds, which is free next to QEMU, but it is a subprocess: on a host where `uptime` is
/// missing or prints an unfamiliar shape, every field stays `None` and the report says "unavailable"
/// rather than guessing.
pub(crate) struct HostLoad {
    min: f64,
    max: f64,
    total: f64,
    samples: u32,
    last: std::time::Instant,
}

impl HostLoad {
    /// Every five seconds. The callers poll on a 100 ms cadence for the scanout referee, and one
    /// `fork`/`exec` per poll would be 3,000 of them over a five-minute leg to resolve a number
    /// that moves on a sixty-second decay.
    const EVERY: std::time::Duration = std::time::Duration::from_secs(5);

    /// Start sampling, taking the first reading now so a leg that fails in its first second still
    /// reports something.
    pub(crate) fn new() -> Self {
        let mut load = Self {
            min: f64::INFINITY,
            max: 0.0,
            total: 0.0,
            samples: 0,
            // Back-dated so the first `sample()` call fires rather than waiting out the interval.
            // `checked_sub` rather than `-`: `Instant` counts from boot on both our platforms, and
            // subtracting past zero is a panic. A machine that booted four seconds ago is a real
            // CI shape, and a harness that panicked there would be a mystery worth more than the
            // one sample it costs to fall back to waiting the interval out.
            last: std::time::Instant::now()
                .checked_sub(Self::EVERY)
                .unwrap_or_else(std::time::Instant::now),
        };
        load.sample();
        load
    }

    /// Take a reading if the interval has elapsed. Cheap enough to call from a 100 ms poll loop or
    /// from a line-at-a-time transcript reader, which is what the two legs do.
    pub(crate) fn sample(&mut self) {
        if self.last.elapsed() < Self::EVERY {
            return;
        }
        self.last = std::time::Instant::now();
        let Some(now) = one_minute_load_average() else {
            return;
        };
        self.min = self.min.min(now);
        self.max = self.max.max(now);
        self.total += now;
        self.samples += 1;
    }

    /// Say what the host was doing, but only when the leg went red. On a green leg this is noise,
    /// and a diagnostic that prints on every run is a diagnostic readers learn to skip.
    pub(crate) fn report_if_failed(&self, ok: bool, arch: &str) {
        if ok {
            return;
        }
        eprintln!();
        if self.samples == 0 {
            eprintln!(
                "host load ({arch}): unavailable (`uptime` did not answer in a shape this parses)"
            );
            return;
        }
        let mean = self.total / f64::from(self.samples);
        let cpus = std::thread::available_parallelism().map_or(0, |n| n.get());
        eprintln!(
            "host load ({arch}): 1-minute average {:.2} / {:.2} / {:.2} (min/mean/peak over {} \
             samples), on {} cores",
            self.min, mean, self.max, self.samples, cpus,
        );
        if cpus > 0 && self.max > cpus as f64 {
            eprintln!(
                "  {:.1}x oversubscribed at the peak. A timing assertion that failed above may be \
                 measuring this machine rather than this kernel; `script/icount` asserts the timer \
                 claims in instructions, which nothing the host does can move. See \
                 notes/load-sensitive-assertions.md.",
                self.max / cpus as f64,
            );
        } else {
            eprintln!(
                "  Not oversubscribed, so contention is the less likely explanation for a failure \
                 above. See notes/load-sensitive-assertions.md."
            );
        }
    }
}

/// The host's one-minute load average, from `uptime`.
///
/// `uptime` rather than `getloadavg(3)` because reaching the libc call means taking the `libc`
/// crate, and §46 makes a dependency a decision rather than a convenience: this is one number, read
/// once every five seconds, on a machine that is already running an emulator. `script/repeat-under-load`
/// parses the same command with the same trick, and this is deliberately the same parse in Rust:
/// macOS prints `load averages: 4.14 4.86 4.29` and Linux prints `load average: 0.50, 0.40, 0.30`,
/// so stripping commas first lets one scan serve both.
fn one_minute_load_average() -> Option<f64> {
    let out = Command::new("uptime").output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_load_average(std::str::from_utf8(&out.stdout).ok()?)
}

/// The pure half of [`one_minute_load_average`], split out so it can be tested on the host without
/// an `uptime` to run.
///
/// **The two formats are not both reachable from one machine**, which is what makes the test worth
/// having rather than filler: development is macOS and CI is `ubuntu-24.04-arm`, so a parse that
/// only understood the shape in front of its author would keep working here and quietly report
/// "unavailable" on every CI run, which is exactly the silence this whole feature exists to end.
fn parse_load_average(uptime_output: &str) -> Option<f64> {
    // Commas out first: Linux separates the three figures with them and macOS does not, so one
    // scan serves both once they are gone. This is `script/repeat-under-load`'s `load_now` in Rust.
    let text = uptime_output.replace(',', " ");
    let mut fields = text.split_whitespace();
    while let Some(f) = fields.next() {
        if f == "average:" || f == "averages:" {
            return fields.next()?.parse().ok();
        }
    }
    None
}

/// **Run the kernel test suite for `arch` and prove BOTH scanouts while it runs.** `test_args` is the
/// cargo invocation the caller would otherwise have handed to [`crate::host::run`].
///
/// **Three** pictures reach the device's scanout over one boot, in this order, because that is the
/// order the suite runs them in (tests sort by name, so `compositor_tests` comes before
/// `display_tests`, and within the latter `a_backing...` < `a_bitmap...` < `a_confined...`):
///
/// 1. rung two's **composed screen** (milestone 33): three clients' surfaces, composited by `compositor`.
///    The compositor test holds it up for a few seconds precisely so this poll cannot miss it;
/// 2. the display terminal's **text** (milestone 29's remaining increment): real glyphs from the
///    `bitmap_font` table, laid out by the `video_terminal` engine. Held up the same way, for the same reason;
/// 3. rung one's **test pattern** (milestone 29), which then stays on the scanout until QEMU exits.
///
/// All three must be seen or the run fails, and the order is part of the check: this looks for each
/// picture until it finds it and only then starts looking for the next. So a reordering of the suite,
/// or a component that never got its picture to the device, fails loudly instead of being waved
/// through. The child inherits stdio, so the suite's output streams exactly as before.
pub(crate) fn cargo_test_with_scanout_check(arch: &str, test_args: &[&str]) -> bool {
    let mut referee = ScanoutReferee::new(arch);
    // The other host-side actor (milestone 107): a process that connects INTO the guest, which is
    // the one thing no in-guest test can stage. Constructed before the child for the same reason
    // the referee is: it is what sets `NIFE_HOSTFWD_PORT`, and the runner reads it from the
    // environment the child inherits.
    let prober = InboundProber::new(arch);
    // Timed and recorded (milestone 807 (the kernel suite reports what each test cost)): stdout
    // passes through as it did when the child inherited it, and the transcript is kept for
    // `time_record::account` below.
    let (mut child, watch) =
        match crate::time_record::Watch::spawn(Command::new("cargo").args(test_args)) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("failed to run cargo: {e}");
                return false;
            }
        };

    // **The suite's verdict is collected, not returned early.** An early return here skipped every
    // prober's report exactly when a guest-side assertion had failed, which is the run where their
    // findings matter most: milestone 55's responder lane spent two five-minute suites learning
    // nothing, because the guest said "nobody ever asked me anything" and the host side, which
    // knew precisely why it had stopped asking, was never given the chance to say so.
    let mut child_ok = false;
    // Sampled here, in the loop that already exists, because the number worth having is the one
    // from *while the leg ran*: a suite takes minutes and the one-minute average has decayed by the
    // time the verdict is in. Reported only if something below goes red.
    let mut load = HostLoad::new();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                child_ok = status.success();
                break;
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("waiting for the test child failed: {e}");
                break;
            }
        }
        referee.poll();
        load.sample();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // All of them, and not short-circuited: a run that lost the scanout AND a network answer should
    // say so once rather than making the reader run it again to find the next failure.
    // **Under `--test` the host-side referees are advisory** (milestone 210). Each asserts
    // something a particular guest test does (pixels on the scanout, an inbound connection
    // accepted), so a filter that did not select that test fails them for a
    // reason that has nothing to do with what was run. They still RUN, because the referee is also
    // what presses keys over the monitor and the keyboard test needs that; only their verdict is
    // dropped. The guest's own verdict (`child_ok`) is never advisory.
    let filtered = std::env::var_os("NIFE_TEST_FILTER").is_some_and(|v| !v.is_empty());
    if filtered {
        eprintln!();
        eprintln!(
            "--- the host-side checks below are ADVISORY under --test: they assert what particular \
             guest tests write, and a filter may not have selected those ---"
        );
    }
    let scanout = referee.report();
    // There were three until 2026-09-15, when milestone 298 retired the multicast DNS responder
    // and the multicast prober that checked its answers (notes/mdns.md).
    let inbound = prober.report();
    let (lines, exited) = watch.finish();
    let image = test_args
        .iter()
        .skip_while(|a| **a != "-p")
        .nth(1)
        .copied()
        .unwrap_or("unknown");
    let accounted = crate::time_record::account(
        &crate::time_record::leg(arch),
        image,
        &lines,
        exited,
        child_ok,
    );
    let ok = child_ok && accounted && (filtered || (scanout && inbound));
    load.report_if_failed(ok, arch);
    ok
}

/// **The host-side referee for one booted suite**: presses a key through QEMU's monitor and watches
/// the device's own scanout for the three pictures the suite puts there.
///
/// It exists as a struct rather than a loop body because **two legs need the same referee driven
/// two different ways** (milestone 81). Under TCG the harness exits by itself, so the loop can be
/// "poll until the child is gone". Under HVF nothing exits (QEMU does not answer the semihosting
/// trap), so the verdict comes from reading the transcript, which blocks, and the referee has to be
/// driven from a second thread beside it. Same state machine, same messages, two drivers.
pub(crate) struct ScanoutReferee {
    arch: String,
    sock: String,
    shot: PathBuf,
    composed_shot: PathBuf,
    text_shot: PathBuf,
    composed: Option<String>,
    text: Option<String>,
    matched: Option<String>,
    last_composed: String,
    last_text: String,
    last_reason: String,
}

impl ScanoutReferee {
    /// Clear last run's evidence and tell the runner where to put the monitor socket.
    pub(crate) fn new(arch: &str) -> Self {
        let sock = gpu_mon_socket(arch);
        let shot = gpu_shot_path(arch);
        let composed_shot = gpu_compose_path(arch);
        let text_shot = gpu_text_path(arch);
        let _ = std::fs::remove_file(&sock);
        let _ = std::fs::remove_file(&shot);
        let _ = std::fs::remove_file(&composed_shot);
        let _ = std::fs::remove_file(&text_shot);

        // SAFETY: `set_var`/`remove_var` became unsafe in edition 2024 because they race other
        // threads. xtask is single-threaded here: this runs on the main thread before the child
        // that reads it is spawned, and the threads xtask ever starts (the transcript reader in
        // swish_check_leg, and this referee's driver in hvf_kernel_leg) copy pipe bytes and poll a
        // socket, and neither touches the environment.
        unsafe { std::env::set_var("NIFE_GPU_MON", &sock) };

        let missing = String::from("no screendump was ever taken (did QEMU get a monitor?)");
        Self {
            arch: arch.to_string(),
            sock,
            shot,
            composed_shot,
            text_shot,
            composed: None,
            text: None,
            matched: None,
            last_composed: missing.clone(),
            last_text: missing.clone(),
            last_reason: missing,
        }
    }

    /// One pass: press a key, take a screendump, and see whether it is the picture we are waiting
    /// for. Call it on a cadence for as long as the suite is running.
    ///
    /// **Costs more per poll than it used to, and that is a considered, recorded choice rather
    /// than an oversight** (milestone 142, 2026-08-27). The scanout grew from 128x64 (8,192
    /// pixels) to 924x344 (317,856 pixels), roughly 39x more data moved through `screendump` on
    /// the same 100 ms cadence. Measured, not assumed: both architectures' full test suites still
    /// completed in normal time with no timeout pressure at the new size. calef confirmed leaving
    /// the cadence unchanged rather than widening it preemptively, since nothing is currently
    /// slow enough to measure a real problem against; revisit if a future resolution increase
    /// (or a slower CI runner) actually makes this cadence cost something observable.
    pub(crate) fn poll(&mut self) {
        // Press a key every poll. Harmless before the keyboard driver exists (QEMU drops the event)
        // and harmless after its test has passed (the driver ends up parked in a `CALL` nobody
        // answers), so there is nothing to time.
        sendkey(&self.sock, video_terminal::script::HOST_KEY);
        if self.matched.is_none()
            && screendump(&self.sock, &self.shot)
            && let Ok(bytes) = std::fs::read(&self.shot)
        {
            // Each picture is transient except the last (the next test on the same device replaces
            // it), so the dump that matched is copied aside: `shot` is overwritten on every poll.
            if self.composed.is_none() {
                match scanout_holds_the_composed_screen(&bytes) {
                    Ok(()) => {
                        let _ = std::fs::write(&self.composed_shot, &bytes);
                        self.composed = Some(format!("{}", self.composed_shot.display()));
                    }
                    Err(reason) => self.last_composed = reason,
                }
            } else if self.text.is_none() {
                match scanout_holds_the_terminals_text(&bytes) {
                    Ok(()) => {
                        let _ = std::fs::write(&self.text_shot, &bytes);
                        self.text = Some(format!("{}", self.text_shot.display()));
                    }
                    Err(reason) => self.last_text = reason,
                }
            } else {
                match scanout_holds_the_pattern(&bytes) {
                    Ok(()) => self.matched = Some(format!("{}", self.shot.display())),
                    Err(reason) => self.last_reason = reason,
                }
            }
        }
    }

    /// Say what reached the device's scanout and what did not, and return whether all three did.
    pub(crate) fn report(self) -> bool {
        let _ = std::fs::remove_file(&self.sock);
        let arch = &self.arch;
        let (composed, text, matched) = (&self.composed, &self.text, &self.matched);
        let (last_composed, last_text, last_reason) =
            (&self.last_composed, &self.last_text, &self.last_reason);

        let mut ok = true;
        match composed {
            Some(path) => eprintln!(
                "scanout check ({arch}): the compositor's {} windows reached the DEVICE's scanout, \
             verified pixel for pixel against compositor::expected_screen_pixel ({path})",
                compositor::SCENE.len(),
            ),
            None => {
                eprintln!();
                eprintln!(
                    "scanout check ({arch}) FAILED: the compositor test passed, so the guest's witnesses \
                 agree about the framebuffer, but QEMU's scanout never held the composed screen. Last \
                 mismatch: {last_composed}"
                );
                eprintln!(
                    "  A compositor's output is exactly what a guest-side digest cannot confirm; this is \
                 the check that can. See notes/compositor.md."
                );
                ok = false;
            }
        }
        match text {
            Some(path) => eprintln!(
                "scanout check ({arch}): the display terminal's text reached the DEVICE's scanout, \
             verified pixel for pixel against the vt engine run over video_terminal::script ({path})",
            ),
            None => {
                eprintln!();
                eprintln!(
                    "scanout check ({arch}) FAILED: the display-terminal test passed, so the guest \
                 agrees about the framebuffer, but QEMU's scanout never held the terminal's text. \
                 Last mismatch: {last_text}"
                );
                eprintln!(
                    "  A wrong pixel format makes a test pattern look odd and makes text unreadable, \
                 which is why this check exists for glyphs too. See notes/glyphs.md."
                );
                ok = false;
            }
        }
        match matched {
            Some(path) => eprintln!(
                "scanout check ({arch}): the {}x{} pattern reached the DEVICE's scanout, verified pixel \
             for pixel against graphics_protocol::pixel ({path})",
                graphics_protocol::WIDTH,
                graphics_protocol::HEIGHT,
            ),
            None => {
                eprintln!();
                eprintln!(
                    "scanout check ({arch}) FAILED: the display test passed, so the framebuffer holds \
                 the pattern, but QEMU's scanout never did. Last mismatch: {last_reason}"
                );
                eprintln!(
                    "  This is the check that catches a wrong pixel format or scanout rectangle, which \
                 the in-guest test cannot see. See notes/framebuffer-contract.md."
                );
                ok = false;
            }
        }
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Both operating systems' `uptime`, because only one of them is ever in front of you.**
    /// Development happens on macOS and CI runs on `ubuntu-24.04-arm`, so the format not under the
    /// author's nose is the one that breaks silently: a failed parse reports "unavailable" rather
    /// than erroring, which is right at run time and useless as a signal.
    ///
    /// The real strings, copied from each system rather than reconstructed.
    #[test]
    fn the_load_average_parse_serves_macos_and_linux() {
        let macos = "11:07  up 5 days, 22:33, 3 users, load averages: 4.14 4.86 4.29";
        let linux = " 18:02:11 up 12 days,  3:41,  2 users,  load average: 0.50, 0.40, 0.30";
        assert_eq!(parse_load_average(macos), Some(4.14));
        assert_eq!(parse_load_average(linux), Some(0.50));
        // A shape nothing here recognises is `None`, not a wrong number: the report says so out
        // loud, and a made-up load average would be worse than no line at all.
        assert_eq!(parse_load_average("up 3 days"), None);
        assert_eq!(parse_load_average("load average:"), None);
        assert_eq!(parse_load_average("load average: n/a"), None);
    }

    /// Build a P6 PPM of the surface's geometry from a per-pixel function, the way QEMU's
    /// `screendump` writes one.
    fn ppm(pixel: impl Fn(u32, u32) -> (u8, u8, u8)) -> Vec<u8> {
        let (w, h) = (graphics_protocol::WIDTH, graphics_protocol::HEIGHT);
        let mut v = format!("P6\n{w} {h}\n255\n").into_bytes();
        for y in 0..h {
            for x in 0..w {
                let (r, g, b) = pixel(x, y);
                v.extend_from_slice(&[r, g, b]);
            }
        }
        v
    }

    fn pattern_rgb(x: u32, y: u32) -> (u8, u8, u8) {
        let w = graphics_protocol::pixel(x, y);
        (
            ((w >> 16) & 0xff) as u8,
            ((w >> 8) & 0xff) as u8,
            (w & 0xff) as u8,
        )
    }

    /// **The scanout check accepts the pattern and rejects everything else.**
    ///
    /// This is the negative control for the milestone-29 scanout proof, and it matters: a checker that
    /// accepted anything would report "the pixels reached the device" on every run, which is exactly
    /// the kind of test that is worse than none. Each rejection below is a real failure mode of a
    /// framebuffer driver: a scanout never set (the default console size), a resource that was never
    /// transferred into (black), a channel order mixed up (the single most common framebuffer bug),
    /// and one wrong pixel.
    #[test]
    fn the_scanout_check_accepts_the_pattern_and_rejects_near_misses() {
        assert!(scanout_holds_the_pattern(&ppm(pattern_rgb)).is_ok());

        assert!(
            scanout_holds_the_pattern(&ppm(|_, _| (0, 0, 0))).is_err(),
            "a black scanout was accepted",
        );

        // Red and blue swapped: what a wrong virtio-gpu format code produces, and precisely what the
        // in-guest test cannot see (the guest's own bytes are unchanged).
        assert!(
            scanout_holds_the_pattern(&ppm(|x, y| {
                let (r, g, b) = pattern_rgb(x, y);
                (b, g, r)
            }))
            .is_err(),
            "a red/blue-swapped scanout was accepted: the format check is not doing anything",
        );

        // Shifted one row: a stride bug.
        assert!(
            scanout_holds_the_pattern(&ppm(|x, y| pattern_rgb(
                x,
                (y + 1) % graphics_protocol::HEIGHT
            )))
            .is_err(),
            "a scanout shifted by one row was accepted",
        );

        // Exactly one wrong pixel, in the middle.
        assert!(
            scanout_holds_the_pattern(&ppm(|x, y| {
                if (x, y) == (64, 32) {
                    (1, 2, 3)
                } else {
                    pattern_rgb(x, y)
                }
            }))
            .is_err(),
            "a scanout with one wrong pixel was accepted",
        );

        // QEMU's default console size, i.e. a scanout that was never set.
        let mut wrong_geometry = b"P6\n640 480\n255\n".to_vec();
        wrong_geometry.extend(std::iter::repeat_n(0u8, 640 * 480 * 3));
        assert!(
            scanout_holds_the_pattern(&wrong_geometry).is_err(),
            "the default 640x480 console was accepted as our 128x64 surface",
        );

        // A dump caught mid-write is not a failure, but it must not be a pass either.
        let short = &ppm(pattern_rgb)[..1000];
        assert!(scanout_holds_the_pattern(short).is_err());
    }

    fn composed_rgb(x: u32, y: u32) -> (u8, u8, u8) {
        let w = compositor::expected_screen_pixel(compositor::SCENE.len(), x, y);
        (
            ((w >> 16) & 0xff) as u8,
            ((w >> 8) & 0xff) as u8,
            (w & 0xff) as u8,
        )
    }

    /// **The composed-screen check accepts the compositor's screen and rejects the ways a compositor
    /// goes wrong** (milestone 33).
    ///
    /// The negative control for the rung-two half of the scanout proof, and the failure modes are
    /// different from rung one's, which is why it needs its own. In particular a **z-order inversion**
    /// and a **missing window** are both pictures made entirely of correct pixels in almost the right
    /// places: exactly the sort of thing a checker written as "is it not black?" would wave through,
    /// and exactly what a compositor gets wrong.
    #[test]
    fn the_composed_check_accepts_the_screen_and_rejects_the_compositors_own_bugs() {
        assert!(scanout_holds_the_composed_screen(&ppm(composed_rgb)).is_ok());

        assert!(
            scanout_holds_the_composed_screen(&ppm(|_, _| (0, 0, 0))).is_err(),
            "a black screen was accepted",
        );

        // Rung one's pattern is not rung two's screen. Both are 128x64 and both are legitimate
        // pictures, so this pins that the two checks cannot be satisfied by the same dump: if they
        // could, the ordering the poll loop relies on would be meaningless.
        assert!(
            scanout_holds_the_composed_screen(&ppm(pattern_rgb)).is_err(),
            "rung one's test pattern was accepted as the composed screen",
        );
        assert!(
            scanout_holds_the_pattern(&ppm(composed_rgb)).is_err(),
            "the composed screen was accepted as rung one's test pattern",
        );

        // **Stacking order inverted**: the bottom-most window covering a pixel wins instead of the top.
        // Every pixel is a real window pixel; only the order is wrong.
        assert!(
            scanout_holds_the_composed_screen(&ppm(|x, y| {
                for (i, win) in compositor::SCENE.iter().enumerate() {
                    if win.rect().contains(x as i32, y as i32) {
                        let w = compositor::window_pixel(
                            i as u32,
                            (x as i32 - win.origin_x) as u32,
                            (y as i32 - win.origin_y) as u32,
                        );
                        return (
                            ((w >> 16) & 0xff) as u8,
                            ((w >> 8) & 0xff) as u8,
                            (w & 0xff) as u8,
                        );
                    }
                }
                composed_rgb(x, y)
            }))
            .is_err(),
            "a screen with the windows stacked in the wrong order was accepted",
        );

        // **One window missing**: the picture as if the last client never committed. This is what a
        // compositor that dropped a commit, or never mapped a surface, produces.
        assert!(
            scanout_holds_the_composed_screen(&ppm(|x, y| {
                let w = compositor::expected_screen_pixel(compositor::SCENE.len() - 1, x, y);
                (
                    ((w >> 16) & 0xff) as u8,
                    ((w >> 8) & 0xff) as u8,
                    (w & 0xff) as u8,
                )
            }))
            .is_err(),
            "a screen missing its top window was accepted",
        );

        // **Windows placed one pixel off**: the classic clipping error, and the reason the crate's
        // rectangle math is host-tested.
        assert!(
            scanout_holds_the_composed_screen(&ppm(|x, y| composed_rgb(
                (x + 1) % compositor::SCREEN_W,
                y
            )))
            .is_err(),
            "a screen shifted one pixel left was accepted",
        );

        // Red and blue swapped, the format bug the guest cannot see.
        assert!(
            scanout_holds_the_composed_screen(&ppm(|x, y| {
                let (r, g, b) = composed_rgb(x, y);
                (b, g, r)
            }))
            .is_err(),
            "a red/blue-swapped composed screen was accepted",
        );
    }

    fn text_rgb(x: u32, y: u32) -> (u8, u8, u8) {
        // Built once and cached rather than once per pixel: `ppm` below calls this per pixel of a
        // 924x344 image (317,856 times), and reconstructing and re-feeding a `Vt` that many times
        // would dominate this test's runtime the moment the grid grew past a few dozen cells.
        static SCREEN: std::sync::OnceLock<video_terminal::Vt> = std::sync::OnceLock::new();
        let screen = SCREEN.get_or_init(|| {
            let mut vt =
                video_terminal::Vt::new(video_terminal::script::COLS, video_terminal::script::ROWS);
            video_terminal::script::scrolled_full_screen(&mut vt, None);
            vt
        });
        let w = screen.pixel(x, y);
        (
            ((w >> 16) & 0xff) as u8,
            ((w >> 8) & 0xff) as u8,
            (w & 0xff) as u8,
        )
    }

    /// **The text check accepts the terminal's screen and rejects text that is wrong** (milestone
    /// 29's remaining increment).
    ///
    /// The negative control that makes the glyph proof mean anything, and its failure modes are the
    /// *terminal's* rather than the compositor's or the driver's. The one that matters most is the
    /// first: **one letter changed**. Everything else on that screen is identical, every glyph is a
    /// real glyph, the layout is right, and the picture is wrong. A checker that could not tell the
    /// difference would report "readable text reached the scanout" for a terminal that drew the wrong
    /// text, which is the failure this whole increment is about not having.
    #[test]
    fn the_scanout_check_rejects_text_that_is_one_letter_wrong() {
        assert!(scanout_holds_the_terminals_text(&ppm(text_rgb)).is_ok());

        // One letter. `glyphs_ok` against `glyphs_0k`: an `o` for a zero, which is the closest pair
        // of glyphs in the font and therefore the hardest case, deliberately.
        let mut typo =
            video_terminal::Vt::new(video_terminal::script::COLS, video_terminal::script::ROWS);
        video_terminal::script::scrolled_full_screen(
            &mut typo,
            Some(video_terminal::script::SCROLL_LINES + 1),
        );
        assert!(
            scanout_holds_the_terminals_text(&ppm(|x, y| {
                let w = typo.pixel(x, y);
                (
                    ((w >> 16) & 0xff) as u8,
                    ((w >> 8) & 0xff) as u8,
                    (w & 0xff) as u8,
                )
            }))
            .is_err(),
            "a screen with one letter wrong was accepted as the terminal's text",
        );

        // **The typing never arrived.** A terminal that rendered an application's output but dropped
        // the keystrokes routed to it draws a picture that is correct as far as it goes.
        let mut no_input =
            video_terminal::Vt::new(video_terminal::script::COLS, video_terminal::script::ROWS);
        no_input.feed(video_terminal::script::GREETING);
        assert!(
            scanout_holds_the_terminals_text(&ppm(|x, y| {
                let w = no_input.pixel(x, y);
                (
                    ((w >> 16) & 0xff) as u8,
                    ((w >> 8) & 0xff) as u8,
                    (w & 0xff) as u8,
                )
            }))
            .is_err(),
            "a screen missing the typed input was accepted",
        );

        // **The rendition ignored.** Every glyph in the right cell, drawn in the default colours: a
        // terminal that parsed SGR as an unknown sequence and swallowed it. The picture is *nearly*
        // right, which is the point.
        let mut plain =
            video_terminal::Vt::new(video_terminal::script::COLS, video_terminal::script::ROWS);
        for &b in video_terminal::script::GREETING
            .iter()
            .chain(video_terminal::script::TYPED)
        {
            // Strip the escape sequences by feeding only what a colour-blind terminal would keep.
            if b != 0x1b {
                plain.feed(&[b]);
            }
        }
        assert!(
            scanout_holds_the_terminals_text(&ppm(|x, y| {
                let w = plain.pixel(x, y);
                (
                    ((w >> 16) & 0xff) as u8,
                    ((w >> 8) & 0xff) as u8,
                    (w & 0xff) as u8,
                )
            }))
            .is_err(),
            "a screen that ignored every rendition was accepted",
        );

        // A blank terminal, which is what a component that came up and drew nothing leaves.
        let blank =
            video_terminal::Vt::new(video_terminal::script::COLS, video_terminal::script::ROWS);
        assert!(
            scanout_holds_the_terminals_text(&ppm(|x, y| {
                let w = blank.pixel(x, y);
                (
                    ((w >> 16) & 0xff) as u8,
                    ((w >> 8) & 0xff) as u8,
                    (w & 0xff) as u8,
                )
            }))
            .is_err(),
            "a blank terminal was accepted as text",
        );

        // And the three pictures on this one scanout are mutually exclusive, which is what makes the
        // poll loop's ordering a real assertion rather than three chances to match once.
        assert!(scanout_holds_the_terminals_text(&ppm(pattern_rgb)).is_err());
        assert!(scanout_holds_the_terminals_text(&ppm(composed_rgb)).is_err());
        assert!(scanout_holds_the_pattern(&ppm(text_rgb)).is_err());
        assert!(scanout_holds_the_composed_screen(&ppm(text_rgb)).is_err());
    }
}
