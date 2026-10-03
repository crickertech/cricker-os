//! **The graphical terminal: a prompt on the display, launched from the swish prompt**
//! (milestone 632 (provisional), calef's 2026-09-30 ruling: *"I don't want graphics at boot and
//! won't for a long time. Change to launching a program from the swish prompt to get graphics;
//! graphics is going to sit there largely unused for some time."*).
//!
//! The boot this program runs on is the plain UART system §26 (the fault endpoint: thread death becomes a message a supervisor holds) records, unchanged. The
//! person at that prompt delegated the display devices by typing this program's name, and the
//! progenitor built the stack from what the shell delegated: `gpu_driver`, `display_terminal`,
//! and either the session's own line discipline with `keyboard_driver` behind it (a virtio
//! keyboard came with the gpu), or nothing but the screen, in which case this program's
//! keystrokes come from the boot's own line discipline over the UART (milestone 192 (a keyboard
//! on real silicon)'s option A, at launch rather than at boot).
//!
//! Two arms, chosen by `x0`, and both are a prompt that echoes what is typed:
//!
//! ```text
//!   arm 0 (a keyboard came):
//!     keyboard_driver ──OP_BYTES──► line_editor ──OP_WRITE──► display_terminal ──► screen
//!            this program ◄──lines── (slot 2)     (slot 1 == slot 2's server)
//!
//!   arm 1 (no keyboard; the UART is the keystroke source):
//!     boot input ──OP_BYTES──► boot line_editor ◄──OP_READRAW── this program
//!            this program ──OP_WRITE──► display_terminal ──► screen
//! ```
//!
//! Arm 1 is the shell's own §227 (how Tab reaches the shell: the shell edits its own line) shape: raw mode on the boot discipline, register-only reads,
//! echo painted by the reader. The discipline's echo goes to the UART console it was built
//! against, so on this arm the screen's echo is this program's to do, byte for byte as the bytes
//! arrive, which is why `$ a` reaches the screen the moment the key is pressed rather than when
//! the line completes.
//!
//! **Ending.** `^C` ends a session on either arm (arm 0's read fails with
//! `FLAG_INTERRUPTED`, the discipline counted it; arm 1 reads it as the raw byte it is), and a
//! line reading `quit` ends one too. Arm 1 puts the boot discipline back the way it found it on
//! the way out; a session the kernel killed cannot, and the shell's §227 fallback (its
//! `BAD_REQUEST` path) takes the discipline back on the next prompt either way.
//!
//! Its whole authority: the result endpoint (slot 0, one word when the session ends), one
//! endpoint that prints to the screen (slot 1) with the page its server reads, one endpoint that
//! owns the keystrokes (slot 2), and, on arm 0 only, the page the discipline delivers lines
//! into. No device, no interrupt, no DMA: a person delegated those to the drivers, not to the
//! prompt that runs on them.
//!
//! # BUGS
//!
//! **Arm 1 has no line editing.** Raw bytes are echoed as they arrive; backspace and the arrows
//! are stored and painted as the control bytes they are, so a mistyped `quit` cannot be corrected
//! and the only edits are `^C` and retyping. The boot prompt's own editor (the §227 engine) is
//! what arm 0 gets for free from its discipline; giving arm 1 a local copy of it is scoped-out
//! follow-on work, not a property of the launch.
//!
//! **Arm 1 turns the boot discipline cooked on its way out, deliberately.** The courteous state
//! is the one it found before the shell went raw, and the shell's own `BAD_REQUEST` recovery
//! takes raw back at its next prompt, which costs that one round trip after every session. A
//! session the kernel killed skips even that, and the same recovery covers it.
//!
//! Name: ratified 2026-10-03 (calef, #1493). Refused `screen` (clashes with `SCREEN_BIT`, the
//! screen-narrowed tail of §106 (an unredirected tail stage's output goes to the screen, not the
//! shell), and with GNU screen), `gate` (the tree's word for a check that fails loudly, about 3,700
//! uses, and the x86 IDT's interrupt gate), `display_session` and `display_console` (`console` is
//! the UART server). It names the whole stack (`gpu_driver`, `display_terminal` and the keyboard),
//! so it survives any later change of VT engine behind `display_terminal`.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68 (code-quality gates: one lint policy)'s ratchet tracks
// (§107 (`missing_docs` moves to the workspace lints, opt-out rather than opt-in)): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use line_editor::proto;
use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{call, exit, send};

/// The result endpoint: one word when the session ends, the shape every reporting program answers.
const REPORT: u64 = 0;
/// **The screen's write endpoint** (slot 1): arm 0's line discipline, arm 1's `display_terminal`.
/// On either arm the contract is `OP_WRITE` over one page, `line_editor::proto`'s
/// control-by-message, bulk-by-shared-page split.
const WRITE: u64 = 1;
/// **The keystroke endpoint** (slot 2): arm 0's line discipline again (it serves both classes on
/// one endpoint, `display_terminal`'s own one-wait-point rule), arm 1's boot line discipline,
/// which this program reads raw.
const KEYS: u64 = 2;

/// The page the write endpoint's server reads. Which frame backs it is the progenitor's business;
/// on arm 0 it is the page the discipline reads (`line_editor`'s `APP_OUT`), on arm 1 the page
/// `display_terminal` reads (its `OUT_VA`). Must match the spawn wiring; the address is ours.
const OUT_VA: u64 = address_space_map::pair_page(0x0000_0000_00a0_0000);
/// The page the discipline delivers completed lines into, read-only. Arm 0 only; arm 1's reads
/// are register-only and no second page is mapped.
const IN_VA: u64 = address_space_map::pair_page(0x0000_0000_00a1_0000);

// SAFETY: the wiring maps OUT_VA read/write before this program runs, on either arm; nothing
// below writes it before a flush-sized batch is composed and never past one page.
const OUT_WINDOW: MappedWindow = unsafe { MappedWindow::new(OUT_VA, 4096) };
// SAFETY: arm 0's wiring maps IN_VA read-only; arm 1 never reads through it at all.
const IN_WINDOW: MappedWindow = unsafe { MappedWindow::new(IN_VA, 4096) };

/// The prompt. Two bytes, the same `$ ` every nife prompt prints, so a reader decoding the
/// scanout needs no second spelling of one program's name.
const PROMPT: &[u8] = b"$ ";

/// The line that ends a session, judged after the editing is done so a half-typed `qui` is just
/// input, exactly as it would be at the swish prompt.
const QUIT: &[u8] = b"quit";

/// Batch the screen's echo the way the shell's own `Echo` does: fill the page, flush with one
/// `CALL`, so a burst of typing is one round trip per page rather than one per byte.
struct Echo {
    used: usize,
}

impl Echo {
    fn put(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.used == 4096 {
                self.flush();
            }
            OUT_WINDOW.w8(self.used as u64, b);
            self.used += 1;
        }
    }

    fn flush(&mut self) {
        if self.used > 0 {
            call(WRITE, proto::req(proto::OP_WRITE, self.used as u64), 0);
            self.used = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(arm: u64, _a1: u64, _a2: u64) -> ! {
    match arm {
        1 => raw_arm(),
        _ => discipline_arm(),
    }
}

/// **Arm 0: a keyboard came with the gpu, so a line discipline did too.** The discipline does
/// the terminal's whole job, cooked `OP_READLINE` (milestone 28 (a solid terminal: the line discipline as a component)'s original contract): it paints
/// the prompt, echoes each keystroke, moves on enter, and hands this program the completed line,
/// so this arm's loop is a prompt delivered, a line read, and nothing painted by the program
/// itself. `quit` and `^C`/EOF (the discipline's flags) are the two ways out.
fn discipline_arm() -> ! {
    loop {
        // The prompt rides the READLINE request's own page: the discipline reads it from there
        // and paints it, which is what keeps the prompt and the echoing cursor in one place.
        for (i, &b) in PROMPT.iter().enumerate() {
            OUT_WINDOW.w8(i as u64, b);
        }
        let (len, flags) = call(KEYS, proto::req(proto::OP_READLINE, PROMPT.len() as u64), 0);
        if flags & (proto::FLAG_EOF | proto::FLAG_INTERRUPTED) != 0 {
            // EOF or `^C`: the session is over either way. The discipline already moved the
            // cursor for the interrupted line's sake; say nothing and end cleanly.
            break;
        }
        let len = (len as usize).min(4096);
        let mut line = [0u8; 4096];
        for (i, b) in line.iter_mut().enumerate().take(len) {
            *b = IN_WINDOW.r8(i as u64);
        }
        if line[..len] == QUIT[..] {
            break;
        }
    }
    end()
}

/// **Arm 1: no keyboard, so the keystrokes are the boot prompt's own, over the UART.** Raw mode
/// on the boot discipline (`OP_RAWMODE`), register-only reads (`OP_READRAW`), echo painted here:
/// the shell's own §227 shape, run against a screen instead of a UART console. The discipline's
/// cooked-mode echo is bypassed in raw mode, so nothing double-echoes on either surface.
fn raw_arm() -> ! {
    // The shell keeps the discipline raw at the prompt (§227 option D), so this is usually
    // already the state; asking anyway is what makes the session correct when it is not.
    call(KEYS, proto::req(proto::OP_RAWMODE, 1), 0);
    let mut echo = Echo { used: 0 };
    echo.put(PROMPT);
    echo.flush();
    let mut line = [0u8; 4096];
    let mut n = 0usize;
    'session: loop {
        let (got, packed) = call(KEYS, proto::req(proto::OP_READRAW, 0), 0);
        if got == proto::BAD_REQUEST {
            // Somebody left the discipline cooked between our reads. Take raw mode back and ask
            // again, the same recovery the shell's own edit loop performs.
            call(KEYS, proto::req(proto::OP_RAWMODE, 1), 0);
            continue;
        }
        let got = (got as usize).min(8);
        let bytes = packed.to_le_bytes();
        for &b in bytes.iter().take(got) {
            if b == 0x03 {
                // `^C` ends the session, and this arm feels it as the byte it is.
                break 'session;
            }
            if b == b'\r' || b == b'\n' {
                if line[..n] == QUIT[..] {
                    break 'session;
                }
                echo.put(b"\n");
                n = 0;
            } else if n < line.len() {
                // Echo as the byte arrives: the round trip this arm exists to prove.
                echo.put(&[b]);
                line[n] = b;
                n += 1;
            }
        }
        echo.flush();
    }
    // Put the boot discipline back the way the prompt found it. A session the kernel killed
    // skips this, and the shell's `BAD_REQUEST` recovery covers it from its side.
    call(KEYS, proto::req(proto::OP_RAWMODE, 0), 0);
    end()
}

/// One word on the result endpoint, then exit: the session ran and is over.
fn end() -> ! {
    send(REPORT, 0, 0, 0);
    exit()
}

user_mode_runtime::panic_handler!();
