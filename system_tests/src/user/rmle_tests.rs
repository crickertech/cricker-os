//! **`rmle` itself, driven with real keystrokes over the raw-keystroke primitive, saving a real
//! file** (milestone 169's second half).
//!
//! The proof shape mirrors [`raw_mode_tests`](super::raw_mode_tests): the test plays the input
//! driver on `rmle`'s terminal, and the file it edits is verified **independently**, by the test
//! opening the same granted directory itself after `rmle` has exited and reading the bytes back,
//! rather than trusting `rmle`'s own report. That is the two-witness discipline this tree already
//! uses for the confined-C test (`c_seam`'s confiner) and the framebuffer test (driver digest and
//! client digest, taken from two different address spaces): neither witness here is `rmle` grading
//! its own homework.

use filesystem_protocol::fixture::tree;
use filesystem_protocol::fs;

use super::*;
use crate::sched;

const CTRL_S: u8 = 0x13;
const CTRL_Q: u8 = 0x11;
const ESC: u8 = 0x1b;

/// Send `text` as the input driver would, in bursts of up to 8 raw bytes, exactly
/// [`raw_mode_tests`]'s own `send_bytes` scaled up for a whole line at a time.
fn type_text(term: sched::RendezvousId, text: &[u8]) {
    for chunk in text.chunks(8) {
        let mut w1 = 0u64;
        for (i, &b) in chunk.iter().enumerate() {
            w1 |= (b as u64) << (8 * i);
        }
        let w0 = line_editor::proto::req(line_editor::proto::OP_BYTES, chunk.len() as u64);
        sched::ipc_call(term, [w0, w1]);
    }
}

/// Bounded wall-clock poll, [`compositor_tests`]'s own `wait_for` shape: the work under test runs
/// concurrently (a real spawned process), so a fixed number of yields on an idle core would return
/// at once and prove nothing.
fn wait_for(mut cond: impl FnMut() -> bool) -> bool {
    let deadline = crate::arch::timer::now() + 2 * crate::arch::timer::frequency();
    while crate::arch::timer::now() < deadline {
        if cond() {
            return true;
        }
        sched::yield_now();
    }
    cond()
}

/// Does `rmle`'s terminal-output page currently contain `needle`, anywhere in its first `len`
/// bytes? Used to wait for a specific redraw (the status line changing to "saved", or the message
/// bar showing the unsaved-changes warning) without assuming a fixed timing.
fn page_contains(phys: u64, len: usize, needle: &[u8]) -> bool {
    let base = mmu::phys_to_virt(phys);
    let mut buf = [0u8; 4096];
    let len = len.min(buf.len());
    for (i, b) in buf[..len].iter_mut().enumerate() {
        // SAFETY: `phys` is rmle's own terminal-output frame, which this test allocated.
        *b = unsafe { core::ptr::read_volatile((base + i as u64) as *const u8) };
    }
    buf[..len].windows(needle.len()).any(|w| w == needle)
}

/// Read a file back through a **fresh** `OPEN` on the same narrowed directory, independent of
/// whatever handle `rmle` held (which is closed and gone by the time this runs). Returns the
/// bytes actually on disk, or `None` if the name is not there.
fn read_file_back(dir: sched::RendezvousId, file_shared: u64, name: &[u8]) -> Option<[u8; 256]> {
    let base = mmu::phys_to_virt(file_shared);
    for (i, &b) in name.iter().enumerate() {
        // SAFETY: `file_shared` is the frame this directory capability shares with its holder;
        // safe to write once nothing else is using it, which is true after `rmle` has exited.
        unsafe { core::ptr::write_volatile((base + i as u64) as *mut u8, b) };
    }
    let w0 = fs::req(fs::OPEN, fs::ROOT, name.len() as u64);
    let r = sched::ipc_call(dir, [w0, 0]);
    if (r[0] as i64) < 0 {
        return None;
    }
    let handle = r[0];
    let w0 = fs::req(fs::READ, handle, 256);
    let r = sched::ipc_call(dir, [w0, 0]);
    let n = (r[0] as i64).max(0) as usize;
    let mut out = [0u8; 256];
    for (i, b) in out.iter_mut().enumerate().take(n.min(256)) {
        // SAFETY: same frame, read after the READ reply, so the server's write already happened.
        *b = unsafe { core::ptr::read_volatile((base + i as u64) as *const u8) };
    }
    sched::ipc_call(dir, [fs::req(fs::CLOSE, handle, 0), 0]);
    Some(out)
}

/// **Open a file, move the cursor, insert and delete characters, save** -- the milestone's own
/// scope statement, each clause driven by a real keystroke and the save verified by an independent
/// read of the real file `rmle` wrote.
///
/// The script: type "helo" (a deliberate typo), arrow left twice to land the cursor between the
/// 'e' and the 'l', insert an 'l' (fixing it to "hello", cursor left mid-word), End to reach the
/// end of the line before Enter (so the split does not carry stray tail text onto the next row),
/// type "rmle", `^S`, `^Q`. If any of cursor movement, mid-line insertion, End, or the newline
/// split were broken, the saved file would not read `"hello\nrmle"`.
#[test_case]
fn rmle_edits_and_saves_a_real_file() {
    let Some((w, held)) = rmle_service::start(tree::SUB, "rmle_edit.txt") else {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    };

    // Wait for rmle's first redraw (rawmode + load + the initial paint all complete by then)
    // before typing anything, so no keystroke can land while the terminal is still in canonical
    // mode from line_editor's own boot default.
    assert!(
        wait_for(|| page_contains(w.term_out_phys, 4096, b"rmle --")),
        "rmle never drew its first screen",
    );

    type_text(w.term, b"helo");
    type_text(w.term, &[ESC, b'[', b'D', ESC, b'[', b'D']); // left, left: cursor between 'e' and 'l'
    type_text(w.term, b"l"); // "helo" -> "hello", cursor now mid-word ("hel|lo")
    type_text(w.term, &[ESC, b'[', b'F']); // End: cursor to end of line before splitting it
    type_text(w.term, b"\r");
    type_text(w.term, b"rmle");
    type_text(w.term, &[CTRL_S]);

    assert!(
        wait_for(|| page_contains(w.term_out_phys, 4096, b"saved")),
        "rmle never reported a successful save",
    );

    type_text(w.term, &[CTRL_Q]);
    let [status, dirty, ..] = sched::ipc_recv(w.report);
    assert_eq!(
        status, 1, /* STATUS_QUIT */
        "rmle did not quit cleanly"
    );
    assert_eq!(dirty, 0, "rmle quit dirty after a save");

    let got = read_file_back(w.dir, w.file_shared, b"rmle_edit.txt")
        .expect("the file rmle saved is not there");
    assert!(
        got.starts_with(b"hello\nrmle"),
        "saved content is wrong: {:?}",
        &got[..10],
    );
    held.release_or_fail("rmle_service");
}

/// **Quitting a dirty document needs a second `^Q`, and refusing the first one does not save.**
///
/// This is the negative control the save test does not cover: without it, "the file has the right
/// content" would be consistent with `rmle` saving on every keystroke rather than only on `^S`.
#[test_case]
fn rmle_refuses_to_quit_dirty_without_confirmation() {
    let Some((w, held)) = rmle_service::start(tree::SUB, "rmle_dirty.txt") else {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    };
    assert!(
        wait_for(|| page_contains(w.term_out_phys, 4096, b"rmle --")),
        "rmle never drew its first screen",
    );

    type_text(w.term, b"unsaved");

    // A read parked on the report endpoint would block the test forever if this `^Q` were
    // (wrongly) enough to quit, so the check is a negative one: the report must NOT have arrived
    // within a bounded wait, and the terminal must show the confirmation prompt instead.
    type_text(w.term, &[CTRL_Q]);
    assert!(
        wait_for(|| page_contains(w.term_out_phys, 4096, b"unsaved changes")),
        "rmle did not warn about unsaved changes on the first ^Q",
    );

    type_text(w.term, &[CTRL_Q]);
    let [status, dirty, ..] = sched::ipc_recv(w.report);
    assert_eq!(status, 1 /* STATUS_QUIT */, "the second ^Q must quit");
    assert_eq!(
        dirty, 1,
        "rmle quit without ever having saved; it must report dirty"
    );

    // And the independent witness: nothing was ever written, so the name rmle created (CREATE, on
    // load, since it did not exist) reads back empty.
    let got = read_file_back(w.dir, w.file_shared, b"rmle_dirty.txt")
        .expect("rmle's load(CREATE) should have made the name exist");
    assert!(
        got.iter().all(|&b| b == 0),
        "quitting without saving must not have written anything",
    );
    held.release_or_fail("rmle_service");
}
