//! The `cargo xtask swish-check` command: boot the shell, type at it, and check what it
//! answered.
//!
//! The script is a table of (command, expected substrings), so adding a line to the check is
//! adding a row rather than writing a test.

use std::process::Command;

use crate::archive::{initrd_path, initrd_riscv, riscv_initrd_path};
use crate::disk::{disk_path, mkdisk, mkredoxfs, redoxfs_server_build};
use crate::host::{flag_value, run, workspace_root};
use crate::scanout::{gpu_mon_socket, scanout_rows, screendump, sendkey};
use crate::suite::ArchLegs;
use crate::uefi::{esp_dir, uefi_image};
use crate::{RISCV_TARGET, RUNNER, TARGET, X86_TARGET, profile_dir, user};

/// **Boot the `--features shell` system and type at it** (milestone 50, notes/pipes.md).
///
/// # Why this exists
///
/// Everything else that exercises the shell wires it from **the kernel**, which serves the spawn
/// protocol in place of `components/src/progenitor.rs`. The shell cannot tell the difference, and
/// that is the problem: a change to the progenitor that broke the spawn path fails nothing. The interactive
/// boot is the only thing that runs the real progenitor, and until this verb existed nothing ran the
/// interactive boot.
///
/// It bit milestone 50 three times in one session and **all three presented as a boot that printed
/// nothing at all**: a virtual-address collision between the shell's terminal page and the page six
/// FS clients map, the progenitor's sixteen-slot capability table overflowing when the kernel handed it two more grants,
/// and four stack pages being one deep call short of the redirection path. Each cost a manual bisect
/// against a live prompt. Each is caught here in one boot.
///
/// # What it types, and why those lines
///
/// The lines answer each other rather than a constant, which is the shape the milestone's guest
/// tests already have:
///
/// ```text
/// echo hello world | wc      -> 1 2 12   the bytes went through a real spawned process
/// echo hello world > gate    -> nothing  the same bytes into a file the shell backs
/// wc < gate                  -> 1 2 12   ... and they are the same bytes
/// echo hello world >> gate   -> nothing
/// wc < gate                  -> 2 4 24   ... exactly twice, so `>>` kept the first line
/// wc gate                    -> 2 4 24   milestone 31: the name IS the grant, same bytes
/// wc                         -> refused  ... and with no name there is nothing to read
/// caps wc gate               -> input     ... and the preview says which file, and how
/// date                       -> ...UTC   a real wall clock, wired through the real progenitor
/// caps date                  -> cap 1    ... and `caps` names the capability that made it real
/// ```
///
/// One line would meet the BUGS entry that asked for this. Five is still seconds, and it walks the
/// whole endowment: a spawn through the real progenitor, the FS service the real progenitor narrowed into the
/// shell, and both redirection operators.
pub(crate) fn swish_check() -> bool {
    let legs = match flag_value("--arch").as_deref() {
        None => ArchLegs::All,
        Some("aarch64") => ArchLegs::Aarch64,
        Some("riscv64") => ArchLegs::Riscv64,
        // The third leg (milestone 182), since milestone 299 gave x86_64 a userspace console to
        // reach a prompt through. It boots under OVMF rather than PVH; see `swish_check_leg`.
        Some("x86_64") => ArchLegs::X86_64,
        Some(other) => {
            eprintln!(
                "swish-check: --arch {other} is not an architecture (aarch64, riscv64 or x86_64)"
            );
            return false;
        }
    };
    // `--graphical` (milestone 177, option A): the same two legs, with the GPU and the keyboard
    // attached instead of the plain UART pair, verified by screendump rather than by transcript.
    // See [`swish_check_leg_graphical`]'s own doc for why this needs a whole different verification
    // shape rather than two env vars added to [`swish_check_leg`].
    let graphical = std::env::args().any(|a| a == "--graphical");
    // **Milestone 192's option A**: the same graphical boot with the *keyboard* left off, so the
    // keystroke source is the board's own UART. See [`swish_check_leg_graphical`]'s own doc.
    let graphical_serial = std::env::args().any(|a| a == "--graphical-serial");

    // TCG only. This boot never exits (the shell loops on its prompt), so it is killed rather than
    // waited on, and there is nothing HVF would buy a gate that spends its time in QEMU's serial.
    // SAFETY: `set_var`/`remove_var` became unsafe in edition 2024 because they race other
    // threads. xtask is single-threaded here: this runs on the main thread before the child
    // that reads it is spawned, and the only thread xtask ever starts (the transcript reader
    // in swish_check_leg, or the graphical leg's own polling loop) copies pipe bytes or polls a
    // socket and never touches the environment.
    unsafe { std::env::remove_var("NIFE_ACCEL") };
    if graphical || graphical_serial {
        // No x86_64 graphical leg: milestone 192's x86 half is not built, and the screen x86_64
        // does have (the firmware's, milestone 400) is read by `cargo xtask uefi-boot` instead.
        if legs == ArchLegs::X86_64 {
            eprintln!(
                "swish-check: there is no graphical leg on x86_64; `cargo xtask uefi-boot` reads \
                 the shell off the firmware's screen"
            );
            return false;
        }
        let keystrokes = if graphical_serial {
            Keystrokes::Serial
        } else {
            Keystrokes::Device
        };
        if legs.aarch64() && !swish_check_leg_graphical(false, keystrokes) {
            return false;
        }
        if legs.riscv64() && !swish_check_leg_graphical(true, keystrokes) {
            return false;
        }
        return true;
    }
    if legs.aarch64() && !swish_check_leg("aarch64") {
        return false;
    }
    if legs.riscv64() && !swish_check_leg("riscv64") {
        return false;
    }
    if legs.x86_64() && !swish_check_leg("x86_64") {
        return false;
    }
    true
}

/// **One line this gate types, what it must answer, and how many jobs it runs.**
///
/// The job count sits beside the line because it is a fact about the line, and until 2026-09-26 it
/// was a hand-kept total in the success summary instead: every lane that added a line had to edit
/// that total and the script's declared length, so any two such lanes conflicted (five rebases in
/// one day: #1318, #1320, #1322, #1329 and #1330). Now the script is a slice, the total is summed
/// from the lines each leg actually typed, and adding a line touches the line.
///
/// `jobs` is how many children the progenitor builds from its bounded job pool for this line: one
/// per program stage that runs. **Zero** for a builtin (`echo`, `ls`, `apropos`, `package`), a
/// `caps` preview, a line refused at the prompt with nothing spawned, an image the activation set
/// refuses, and a supervised job (`interrupt_heeder` and `interrupt_ignorer`, built from the
/// shell's own untyped rather than the pool). A pipeline counts each program stage, so
/// `wc gate.txt | wc` is two; a `builtin | program` pipeline is one. `rm` with its caretaker is one
/// job whose region holds two processes. It is a required field with no default, so a line cannot
/// be added without saying.
///
/// **The count is checked, not only printed.** A line with `jobs > 0` whose answer carries the
/// shell's "could not spawn" or "faulted" sentence fails the gate even when its wanted phrases are
/// empty (`uuid > id.txt`), which is what makes "ran N jobs" in the summary a statement this gate
/// verified. What it cannot catch is a tag that is too low, since nothing at the prompt reports a
/// job that nobody counted; the host test `a_job_count_names_a_program` bounds a tag from above.
pub(crate) struct Line {
    pub(crate) typed: &'static str,
    pub(crate) jobs: u8,
    pub(crate) answer: &'static [&'static str],
}

/// **What the shell prints when a job it asked for never answered**, either half:
/// `components/src/swish.rs` prints the first when the progenitor could not build the job, and the
/// second is `swish::FAULTED_SENTENCE`, for a job that trapped first. Spelled here rather than
/// imported because xtask does not depend on `swish`, and a phrase is enough for a transcript.
const JOB_DID_NOT_RUN: [&str; 2] = [
    "could not spawn",
    "that command faulted and was killed before it answered",
];

/// A [`Line`], positionally, so the script reads as the prompt does. The name is provisional.
const fn line(jobs: u8, typed: &'static str, answer: &'static [&'static str]) -> Line {
    Line {
        typed,
        jobs,
        answer,
    }
}

/// **What the second boot types** (milestone 198 (a package manager) rung 3a): the installed
/// program still runs after a reboot, a removal takes its vouch away without deleting it, and a
/// rollback vouches for it again. The disk is the only thing the first boot hands this one.
///
/// A removed program is refused only at a prompt that does not hold the run-unvouched capability
/// (DECISIONS §219 gate D2). The boot prompt holds it, provisionally, so here the removal shows in
/// `caps` and the bytes still run; before D2 this script typed the refusal.
///
/// `greeting` rides along (milestone 198 rung 3a's fetch): it was installed as generation 2, it
/// runs after the reboot, and removing `uptime` leaves it running, because a generation drops one
/// program and not its neighbours.
///
/// **The numbers skip one** because the first boot vouched for a build as generation 3 and rolled
/// it back (DECISIONS §221). A generation is never rewritten, so the removal is generation 4, and
/// a rollback is by number (`notes/packages.md`'s BUGS), so it lands on 3: the vouch's generation,
/// which lists `uptime` too.
const SWISH_CHECK_AFTER_REBOOT: &[Line] = &[
    line(1, "packages/uptime/0.1.0/uptime", &["up "]),
    line(
        1,
        "packages/greeting/0.1.0/greeting",
        &[
            "hello from a package this image never carried",
            "clock: held at slot 1, as its manifest note asked",
        ],
    ),
    line(
        0,
        "package remove uptime",
        &["removed; generation 4 is live"],
    ),
    // **Removed means unvouched, not unrunnable, for a session holding D2** (DECISIONS §219 gate
    // D2). Until D2 this line was a refusal. The boot prompt now holds the run-unvouched
    // capability (provisionally), so the bytes still run, with the ruling's endowment rather than
    // the installed manifest; `caps` is what shows the vouch is gone.
    line(
        0,
        "caps packages/uptime/0.1.0/uptime",
        &[
            "provenance: unvouched (digest ",
            "runs on this session's capability to run unvouched bytes",
        ],
    ),
    line(1, "packages/uptime/0.1.0/uptime", &["up "]),
    line(
        1,
        "packages/greeting/0.1.0/greeting",
        &[
            "hello from a package this image never carried",
            "clock: held at slot 1, as its manifest note asked",
        ],
    ),
    line(
        0,
        "package rollback",
        &["rolled back; generation 3 is live"],
    ),
    line(1, "packages/uptime/0.1.0/uptime", &["up "]),
];

/// The text this gate types and what each line must answer. `None` is a line whose answer is
/// checked by a later one rather than by itself, which is every line that writes a file.
///
/// `hello world` plus the newline `echo` adds is twelve bytes; the append arm is exactly twice
/// that. The numbers are spelled out here rather than derived because this is a **boot** gate: if
/// the arithmetic and the boot were both wrong, deriving one from the other would hide it.
const SWISH_CHECK_SCRIPT: &[Line] = &[
    line(1, "echo hello world | wc", &["1 2 12"]),
    line(0, "echo hello world > gate.txt", &[]),
    line(1, "wc < gate.txt", &["1 2 12"]),
    line(0, "echo hello world >> gate.txt", &[]),
    line(1, "wc < gate.txt", &["2 4 24"]),
    // **Milestone 31's headline, at the one interface a human touches**: naming a resource in a
    // command IS granting it. The answer has to be the same as the `<` above it, because it is the
    // same designation with the operator left out, and the pair is what makes that a claim rather
    // than an assertion: one line reaches the file through an operator and one through a name, so
    // if they disagree, one of them opened something else.
    line(1, "wc gate.txt", &["2 4 24"]),
    // **And the same name at the head of a pipeline**, which is the line that answered nothing at
    // all until milestone 50's draining lane. An input operand is resolved by the planner, and the
    // shell used to wire a pipeline's head off the `Line` (which has no `<` on it), so the planned
    // source was dropped and the stage counted an empty stream (a `recv` on an empty slot answers
    // `NoSuchSlot`, which reads as end of document). Two spawned processes, and this shell feeds the
    // first.
    //
    // `2 4 24` plus a newline is seven bytes and three words on one line, so the answer is the
    // answer above it counted. Spelled out rather than derived for this file's reason: it is a boot
    // gate, and deriving one number from another would hide the case where both are wrong.
    line(2, "wc gate.txt | wc", &["1 3 7"]),
    // The negative control the pair would be weaker without. `wc` alone is refused **at the
    // prompt**, before anything is spawned, because its manifest declares that it reads a stream;
    // on Unix the same command is a shell that appears to hang. So the line above granted
    // something, rather than falling back on a default.
    line(0, "wc", &["name a file"]),
    // And `caps` says which file and how, which is the honest half: the shell reads it and streams
    // it in, so what the child holds is an endpoint and not a capability naming the disk.
    line(0, "caps wc gate.txt", &["input    gate.txt"]),
    // **Milestone 40 at the same interface.** `doc` is in the image, is spawnable, and declares that
    // it reads a stream, so bare `doc` is refused at the prompt before anything is spawned, exactly
    // as `wc` is and for the same reason: a viewer that could open the page it renders could open
    // any page.
    line(0, "mdr", &["name a file"]),
    // **The named file reaches the viewer and comes back rendered**, which two of this gate's own
    // comments said it did not until 2026-08-18. Both halves of that were fixed elsewhere and the
    // record was never corrected: the input operand now comes off the plan rather than off the
    // `Line` (`components/src/swish.rs`, the same fix `wc gate.txt | wc` above pins), and
    // `MAX_OUTPUT_CHUNKS` is 4096 rather than the 32 that would have truncated a page to 512 bytes.
    //
    // The numbers are the assertion and not decoration. `gate.txt` is 2 lines, 4 words, 24 bytes
    // (`wc gate.txt`, above). What comes back is **1 line, 4 words, 26 bytes**: the two source
    // lines are one paragraph re-flowed to one output line, and the two bytes are the body indent.
    // A viewer handed an empty stream would answer `0 0 0`, which is what this line answered when
    // the operand was being dropped, so the count is what separates rendering from silence.
    line(2, "mdr gate.txt | wc", &["1 4 26"]),
    // **And the line a person actually wants now renders**, which is milestone 40's whole
    // remaining phase (DECISIONS §106, 2026-08-22). `mdr gate.txt` alone used to make this shell
    // both the writer and the reader of one line, refused rather than hung, because it has one
    // wait point; see `grant_plan::check_chain` and notes/documentation.md for the refusal this replaced.
    // Now the render defaults to `terminal_sink_caretaker` instead of this shell's own result
    // endpoint, so there is no second reader for the shell to wait behind and the page appears at
    // the prompt with no `| wc` in front of it. The text is the same paragraph `mdr gate.txt | wc`
    // counted three lines up, reflowed and indented by the renderer: `gate.txt`'s two source lines
    // become the one line, four words, twenty-six bytes that count asserted, and this line checks
    // the words themselves arrived rather than merely being countable.
    line(1, "mdr gate.txt", &["hello world hello world"]),
    // **The negative control on the viewer itself**, and it is the whole milestone in one screen: a
    // documentation viewer is exactly the program a reader expects to go and fetch things, and this
    // one is handed a stream. `caps` prints what would be granted before anything is spawned, and
    // there is no file capability, no directory and no filesystem endpoint in it. The manifest is
    // byte-identical to `wc`'s, which is why the assertion is the same string.
    line(0, "caps mdr gate.txt", &["input    gate.txt"]),
    // **Milestone 40 phase 2, at the same interface**: the documentation store is installed, and a
    // search of it answers with pages a person can then open.
    //
    // `doc/bundles` is the manifest the search reads, and it is checked as a *file* first, with an
    // ordinary designation, because that is the claim underneath everything below it: the store is
    // real, it is where the reader thinks it is, and nothing special is needed to read it. The
    // numbers are the four bundle names and their newlines, so a bundle added to `DOC_BUNDLES`
    // fails here, which is right: the manifest is what the guest enumerates by.
    line(1, "wc doc/bundles", &["4 4 25"]),
    // **The query a person would type, against shards built from this repository's own markdown.**
    // Two bundles, so the answer is the merge across shards rather than one shard's list, and both
    // named pages are ones a reader wanting to know what a capability is here would want. The
    // counts are deliberately not asserted: they move whenever the notes are edited, and the claim
    // is which pages were found, not how often the word appears in them.
    line(
        0,
        "apropos capability",
        &["doc/swish/pipes.md", "doc/kernel/ipc-naming.md"],
    ),
    // The negative control, and the word is chosen to appear in **no bundled page**. See
    // notes/documentation.md's BUGS for why this one cannot be written into the note that documents it:
    // that note is itself in the store, so a word written there is a word the store then says.
    line(
        0,
        "apropos photosynthesis",
        &["no page in the store says photosynthesis"],
    ),
    // And a search with nothing to search for is refused, in the same sentence every other verb
    // that needs an operand uses.
    line(0, "apropos", &["name what you mean"]),
    // **The payoff, and the reason a search may be a builtin at all.** The name the search printed
    // is an ordinary designation: this line grants `wc` exactly that one page out of the store and
    // nothing else, resolved by the shell against the directory it holds. So search produced a
    // *name*, and the authority moved on the line where a person typed it. A search that had
    // handed a program the store's directory would have moved it three lines earlier and silently.
    line(
        0,
        "caps wc doc/kernel/ipc-naming.md",
        &["input    ipc-naming.md"],
    ),
    // **The clock, from the prompt** (milestone 51's wiring). The answer cannot be a constant, so
    // the check is the one word that separates a real time from both ways of not having one:
    // `Format::Human` ends in the offset's name and the two unknown-clock sentences ("the machine
    // has no clock it believes" / "this process holds no clock capability") contain no `UTC` at
    // all. So this fails if the clock service did not run, if the kernel granted the progenitor no page, if
    // the progenitor did not endow `date`, or if `date` was handed a page nobody published to.
    line(1, "date", &["UTC"]),
    // And the visibility surface agrees with the wiring. `caps` is the only thing in this system
    // that claims to print a process's whole authority, so a clock endowed and not printed would
    // make that claim false. Its wording is host-tested; this proves the wording is about a
    // capability the boot really moves.
    line(0, "caps date", &["cap 1  frame     clock"]),
    // **The inert-configuration page, from the prompt** (milestone 47's environment-variable fork,
    // DECISIONS §111). `date`'s own proof, one manifest field over: this fails if the kernel
    // granted the progenitor no config page, if the progenitor did not endow `printenv`, or if the page's validated
    // domains rejected the boot's own defaults, none of which a host test can see, because
    // `crates/system_initializer`'s spawn wiring is provable only against a real progenitor
    // (this file's module doc names `script/swish-check` as exactly that gate).
    line(1, "printenv", &["TZ=UTC", "LANG=C", "TERM=dumb"]),
    // And the visibility surface agrees with the wiring, `date`'s own check repeated for `config`:
    // `caps` claims to print a process's whole authority, so a config page endowed and not printed
    // would make that claim false.
    line(0, "caps printenv", &["cap 1  frame     config"]),
    // **`ps`, at the real prompt** (milestone 126). The listing itself: a header, and at least the
    // row for `ps` itself, which is a member of the domain the progenitor spawned it into. Asserting the
    // header rather than a tid is deliberate: a tid is a generational name that moves with how many
    // jobs ran before it, and a gate that pinned one would be pinning the boot's history.
    line(1, "ps", &["TID  STATE"]),
    // **`ps` cannot see the machine, and this is the shape of the evidence at the prompt.** The
    // listing above is short: at this line the shell's domain holds `ps` itself and whatever else
    // the shell has running, which is nothing. A `/proc`-shaped `ps` would be listing the progenitor, the
    // shell, the terminal, the FS server, the compositor, the net stack and every driver.
    //
    // **The count is deliberately not asserted here.** `ps | wc` answered three lines on one run
    // and two on the next, because a pipeline spawns both stages into the same domain and whether
    // `ps` walks before or after `wc` exists is a race. That is truthful (a survey is a snapshot,
    // notes/process-view.md) and it makes a count a bad gate. The confinement claim is asserted
    // deterministically and on both ISAs in `kernel::user::survey_tests`, which builds the domain
    // it measures instead of inheriting one.
    // And `caps ps` prints the scope **before** anything is spawned, which is the half Linux has no
    // way to express: there, "which processes can this see" has one answer for every program on the
    // machine and no command line chose it.
    line(0, "caps ps", &["cap 7  endpoint  domain"]),
    // **`pgrep`, at the real prompt** (milestone 126), and what is asserted is deliberately not the
    // tids. `pgrep` prints nothing but names, one per line, and a tid is a generational name that
    // moves with how many jobs ran before it; a gate that pinned one would be pinning the boot's
    // history. So the claim is made through the **second stream**, which is the same trick the four
    // `date 2>` lines above use: `pgrep`'s diagnostics carry a sentence in exactly three cases (the
    // walk was refused, the selector can never match, or nothing matched), so an *empty* second
    // stream is the assertion that none of the three happened. This one line fails if the progenitor endowed
    // no domain, if it endowed one the kernel refuses, or if the filter came back empty.
    line(1, "pgrep 2> pgrep.txt", &[]),
    line(1, "wc < pgrep.txt", &["0 0 0"]),
    // And the asymmetry, printed before anything is spawned, which is the whole of what milestone
    // 126 has instead of the `pgrep`-beside-`pkill` comparison it originally promised. Two phrases:
    // the right (`ENUMERATE`, not `READ`, so the finder cannot receive a death or collect a corpse)
    // and the sentence that says so in English. There is no `caps pkill` line to put beneath this
    // one, because a tid is a name and no method turns one into authority.
    line(
        0,
        "caps pgrep",
        &["cap 7  endpoint  domain   ENUMERATE", "do nothing to them"],
    ),
    // **`top`, at the real prompt** (milestone 282 (a thread's CPU time, and the `top` it makes possible)), and what is asserted is the two things a
    // boot cannot make untrue: the summary line's opening, which proves the ambient uptime counter
    // and the domain count both reached the output stream, and the `TIME(ms)` header, which proves
    // the **second** walk happened. A `top` whose CPU-time walk was refused would print the table
    // without that column and this line would fail, which is the one failure a
    // `pgrep`-style empty-diagnostics check could not catch.
    //
    // The tids and the figures are deliberately not pinned. A tid is a generational name that moves
    // with the boot's history, and a CPU figure is a measurement of a real machine; a gate that
    // pinned either would be pinning this boot rather than the program.
    line(1, "top", &["up ", "threads: ", "TID  STATE     TIME(ms)"]),
    // And the second stream is empty, the same trick the `pgrep 2>` line above uses: `top`
    // complains in exactly the cases `ps` does, so an empty second stream says none of them
    // happened and the table above it is the domain.
    line(1, "top 2> top.txt", &[]),
    line(1, "wc < top.txt", &["0 0 0"]),
    // The scope, printed before anything is spawned. `top` holds `ps`'s three capabilities and not
    // one more: the ranking costs no authority, because the CPU figures are a second walk of the
    // same endpoint under the same right.
    line(0, "caps top", &["cap 7  endpoint  domain   ENUMERATE"]),
    // **`uptime`, at the real prompt** (milestone 126). No domain, no clock: the manifest is
    // `least_authority_demo`'s, because `monotonic_nanos` is granted to every process unconditionally
    // (kernel/src/arch/*/timer.rs's exception to DECISIONS §10). A green line here proves the
    // program was loaded, measured, granted its report endpoint and actually ran at EL0; the exact
    // elapsed time is not asserted because a real boot's timing is not this check's business.
    line(1, "uptime", &["up "]),
    // **The installer** (milestone 198 (a package manager) rung 3a, DECISIONS §208 (installing a
    // package is granting it, and the activation set is versioned)). A package with one byte of
    // its program flipped, and its table of contents rewritten to agree, is refused by the image's
    // catalogue before anything is written: the catalogue is the only thing that can tell
    // (`disk::stage_installed`). Nothing is installed afterwards: the line after says generation 1.
    line(
        0,
        "package install downloads/tampered.nifepkg",
        &["refused: this image's catalogue does not vouch for those bytes; nothing is installed"],
    ),
    // The genuine package, whose digest the image's catalogue carries: the progenitor writes the
    // program under `packages/<stem>/`, writes generation 1, and renames `current` onto it.
    line(
        0,
        "package install downloads/uptime.nifepkg",
        &["installed; generation 1 is live"],
    ),
    // **And what it installed runs, by its bytes** (DECISIONS §219 (how the shell names an
    // installed program to the spawner) option D). A path, so the shell reads the file into frames
    // and the progenitor hashes its own copy and finds the digest in the generation just written.
    // `up ` is the proof it ran: a refusal prints no such thing.
    line(1, "packages/uptime/0.1.0/uptime", &["up "]),
    // **`caps` names who vouched** (§219: "or the source that vouched"): the digest the shell
    // hashed is in the generation the install just wrote.
    line(
        0,
        "caps packages/uptime/0.1.0/uptime",
        &[
            "provenance: vouched by activation generation 1 (digest ",
            // **No note, the default** (milestone 597 (a program carries its manifest in an ELF
            // note), provisional): `uptime` carries no manifest note, so it is bound and endowed as
            // `grant_plan::NO_NOTE_MANIFEST`, its output alone.
            "it carries no manifest note, so it asks for its output and nothing else",
        ],
    ),
    // **And bytes nobody installed, previewed** (§219 gate D2): no generation lists them, and the
    // boot prompt holds the run-unvouched capability (its provisional grant,
    // `crates/system_initializer`), so the preview is the ruling's endowment and says why it runs.
    line(
        0,
        "caps installed/unvouched",
        &[
            "cap 1  page      clock",
            "cap 2  page      config",
            "provenance: unvouched (digest ",
            "runs on this session's capability to run unvouched bytes (slot 22)",
            // **What the note asks, beside what is granted** (milestone 597, provisional). The
            // witness's note asks for the three authorities it probes, and §219 says an unvouched
            // note grants nothing: the rows above are the ruling's three and no more.
            "its manifest note asks for: output bytes, entropy, the network, the process domain",
            "and its note grants nothing",
        ],
    ),
    // **And run: milestone 202 (every confinement test is a ritual until somebody breaks the confinement)'s claim that an unvouched child holds no capability the caller did
    // not delegate** (DECISIONS §219 gate D2). The bytes are `unreachable_network_witness`
    // stripped, so no table vouches for them and they run on the session's capability alone. The
    // line delegated the output and nothing else; the ruling adds the clock and configuration
    // pages. So each of the three authorities the progenitor holds must answer "refused", and the
    // census must read exactly slots 0, 1 and 2. Falsified once per authority before it was
    // committed: granting the domain, entropy or the network to an unvouched child turned this
    // line red (design/roadmap/202's block records the three runs).
    line(
        1,
        crate::disk::INSTALLED_UNVOUCHED,
        &[
            "network: refused (no capability at slot 10)",
            "entropy: refused (no capability at slot 9)",
            "domain: refused (no capability at slot 7)",
            "slots held: 0 1 2\n",
        ],
    ),
    // **A note that asks more than its vouch allows is refused** (milestone 597, provisional).
    // `least_authority_demo`'s note declares an argument, which only a command line designates,
    // and unvouched bytes may hold only what the D2 ruling names. The shell binds the line
    // against the note (it cannot know the verdict), so the refusal is the progenitor's, on its
    // own copy, and nothing is built. `caps` says so first.
    line(
        0,
        "caps installed/asks-an-arg 5",
        &["more than unvouched bytes may hold"],
    ),
    line(
        0,
        "installed/asks-an-arg 5",
        &["the progenitor read those bytes' own manifest, and it does not allow this line"],
    ),
    // **A note that cannot be read is refused, not ignored** (milestone 597): the witness again,
    // with its note's version word set to 9 (`disk::with_note_version`). Running it as if it
    // carried no note would endow a program with a manifest it did not declare.
    line(
        0,
        crate::disk::INSTALLED_MALFORMED_NOTE,
        &["carries a manifest note that cannot be read"],
    ),
    // **Fetching, refused before the network is touched**: the catalogue names no such package,
    // so nothing is asked of the package source. Runs on all three legs, because x86_64's missing
    // NIC is asked about only after the catalogue is.
    line(
        0,
        "package install nosuch",
        &[
            "refused: this image's catalogue names no such package, so nothing was fetched; \
             generation 1 is live",
        ],
    ),
    // **A lying package source.** The gate serves this leg a copy of `uptime` whose program has one
    // byte flipped and whose table of contents agrees (`disk::stage_installed`), under the genuine
    // name: a whole, well-formed HTTP exchange of a well-formed package. Only the image's
    // catalogue can refuse it, and nothing is written.
    line(
        0,
        "package install uptime",
        &["refused: this image's catalogue does not vouch for those bytes; generation 1 is live"],
    ),
    // **Fetched over the booted system's network and installed** (rung 3a's first gap): the
    // progenitor finds `greeting`'s stem in the catalogue, fetches it from the gate's package
    // source through the stack it built at boot, and installs what arrived as it installs a file.
    line(
        0,
        "package install greeting",
        &["fetched and installed; generation 2 is live"],
    ),
    // x86_64 has no NIC, so it installs the same package from the disk instead; the two legs that
    // fetch omit this line ([`swish_check_omits`]). Either way generation 2 is the same table.
    line(
        0,
        "package install downloads/greeting.nifepkg",
        &["installed; generation 2 is live"],
    ),
    // **And it runs** (rung 3a's second gap): bytes no boot image carries, vouched only by the
    // generation just written. The line it prints is its own; no program in the image prints it.
    // That the image lacks `greeting` is checked on the host before the boot, by reading the
    // archive (`disk::stage_installed`), because the prompt cannot tell: `greeting` is no
    // `grant_plan::Prog`, so its bare name is refused whether or not the archive packs it.
    line(
        1,
        "packages/greeting/0.1.0/greeting",
        &[
            "hello from a package this image never carried",
            "clock: held at slot 1, as its manifest note asked",
        ],
    ),
    // **And the manifest it runs with is the one it carries** (milestone 597, provisional): a
    // vouched note is honoured, which the `clock: held` line above proves at the child, and `caps`
    // shows here at the prompt.
    line(
        0,
        "caps packages/greeting/0.1.0/greeting",
        &[
            "clock    read-only. it can read the time",
            "its manifest note asks for: output bytes, the clock",
        ],
    ),
    // **The owner vouches for a local build** (DECISIONS §221 (the boot prompt is the owner's
    // console), ruling 1). `installed/unvouched` is the fresh build the D2 lines above ran on the
    // ruling's endowment (slots 0, 1 and 2). Vouching writes a generation that lists its digest,
    // so the same bytes now run vouched, with the installed manifest (`uptime`'s until a manifest
    // travels in the executable: `grant_plan::INSTALLED_MANIFEST_OF`), which holds the output and
    // nothing else. So the census drops to slot 0: the grant changed with the vouch, and nothing
    // else on the line did. A rollback takes the vouch away and the census is D2's again.
    line(
        0,
        "vouch installed/unvouched",
        &["vouched; generation 3 is live"],
    ),
    line(
        0,
        "caps installed/unvouched",
        &["provenance: vouched by the owner in activation generation 3 (digest "],
    ),
    line(
        1,
        crate::disk::INSTALLED_UNVOUCHED,
        &[
            "network: refused (no capability at slot 10)",
            "entropy: refused (no capability at slot 9)",
            "domain: refused (no capability at slot 7)",
            "slots held: 0\n",
        ],
    ),
    line(
        0,
        "package rollback",
        &["rolled back; generation 2 is live"],
    ),
    line(
        0,
        "caps installed/unvouched",
        &["provenance: unvouched (digest "],
    ),
    line(
        1,
        crate::disk::INSTALLED_UNVOUCHED,
        &["slots held: 0 1 2\n"],
    ),
    // **`uuid`, at the real prompt** (milestone 111), and this is the only gate that can run it.
    // The endowment is `crates/system_initializer`'s to make: the progenitor holds the entropy service's
    // request endpoint and places a `WRITE` view of it at `grant_plan::ENTROPY_SLOT` for a child
    // whose manifest declares it, exactly as it places the clock. Every other test that runs the
    // shell has the KERNEL play the progenitor, and `Spawn` fills a capability table from slot 0 upward, so
    // nothing else in this tree can put a capability at a slot a manifest names.
    //
    // **The value is deliberately not asserted, and the shape is.** A version-4 identifier that a
    // gate could predict would be a version-4 identifier drawn from nothing, so pinning one would
    // assert the opposite of what this milestone is about. What is pinned is the framing: `uuid >
    // id.txt` puts 36 characters and a newline in a file, and `wc` counts one line, one word, 37
    // bytes. A `uuid` the progenitor endowed nothing would leave that file **empty** (its refusal goes to the
    // second stream, `Manifest::output`'s whole reason here), so `1 1 37` fails on exactly the
    // condition this milestone exists to create.
    line(1, "uuid > id.txt", &[]),
    line(1, "wc < id.txt", &["1 1 37"]),
    // And the second stream is empty, which is the same trick the `pgrep 2>` line above uses: this
    // program complains in exactly one case (it holds no entropy capability), so an empty second
    // stream is the assertion that the case did not happen. The two lines together are "it drew
    // real bytes and had nothing to complain about", said without asserting any byte of them.
    line(1, "uuid 2> ent.txt", &[]),
    line(1, "wc < ent.txt", &["0 0 0"]),
    // And the visibility surface, which is what a person meets before anything is spawned. On
    // Linux there is nothing here to say: no tool reports whether a program will read
    // `/dev/urandom`, and nothing about running one reveals it either. Here it is a row, and the
    // right on it (`WRITE`, so it may ask the service and may not receive another client's
    // request) is the claim rather than decoration.
    line(
        0,
        "caps uuid",
        &[
            "cap 9  endpoint  entropy  WRITE",
            "draws no randomness at all",
        ],
    ),
    // **`2>`, at the one interface a human touches** (DECISIONS §67). The four lines below are the
    // whole of the decision, and only this gate runs them through the real progenitor: the guest tests
    // wire the shell from the kernel, whose `Spawn` fills a capability table from zero and cannot place a
    // capability at the slot a manifest names, so `date` there never receives a second stream.
    //
    // `date` is the declarer, and at *this* prompt it has a clock and nothing to complain about. So
    // the assertion is that its second stream exists, is separate, and is **empty**: `2> err.txt`
    // creates the file, `date` closes the stream with nothing on it, and `wc` counts zero of
    // everything. A shell that had merged the two streams would put a timestamp in there.
    line(1, "date 2> err.txt", &["UTC"]),
    line(1, "wc < err.txt", &["0 0 0"]),
    // And the visibility surface names the second destination, which is what stops `caps date >
    // when.txt` being a half-truth: two destinations on one line, and a reader can see that the
    // complaint is not going into the file.
    line(0, "caps date 2> err.txt", &["diags    err.txt"]),
    // The refusal, which is the other half of "a declaration, not a number". `wc` writes one stream
    // and its diagnostics ride it, so `2>` names nothing and the line does not run.
    line(0, "wc gate.txt 2> err.txt", &["declares no second output"]),
    // **`time`, at the one interface a human touches** (milestone 86). Only this gate runs the real
    // inits, and the clock the shell times with is granted by them: the guest tests wire it from the
    // kernel, so a boot where the progenitor never handed the shell a clock would pass every one of those and
    // print "this shell holds no clock capability" here.
    //
    // The answer is the same three numbers `wc gate.txt` gave four lines up, which is the claim the
    // milestone rests on: the tail runs exactly as typed and timing it changes nothing about it. A
    // `time` that re-tokenized its tail, or spawned a differently endowed child, would answer
    // something else here and the duration would still look fine.
    line(1, "time wc gate.txt", &["2 4 24"]),
    // And the duration itself, checked for its shape rather than its value: the number is a real
    // measurement and cannot be a constant, but `real` and a unit are what a stopwatch prints.
    line(1, "time date", &["time: real"]),
    // The visibility surface agrees with the wiring, the same pairing `caps date` makes for the
    // child's clock. This one is about the shell's own: `caps` is the only thing in this system that
    // claims to print a process's whole authority, and a clock the boot really grants would make
    // that claim false if it went unprinted. The rights half is the load-bearing word: READ without
    // GRANT is why nothing typed here can hand a clock to a child.
    //
    // **The second wanted phrase is milestone 31 phase 3's**, and it is the machine-checked form of "flip
    // `holdings()`": the shell's `holdings().dir` is true exactly when the progenitor granted it a directory,
    // and this row is the only place a person can read that at the real prompt. Every other test
    // that runs the shell has the kernel play the progenitor, so a boot that stopped granting it would fail
    // nothing; `wc gate.txt` above would keep working, because the shell opens that file itself.
    line(
        0,
        "caps",
        &[
            "frame     clock      READ only, NOT delegable",
            "endpoint  directory",
        ],
    ),
    // **Milestone 31 phase 3, at the one interface a human touches** (2026-08-17). Naming a
    // directory in a command IS granting a capability to it, and until this landed the prompt could
    // say so and not do it: the progenitor deleted the file service during the boot, so a directory grant had
    // nothing to build a caretaker out of and `rm` was a refusal.
    //
    // Four lines, and they are one argument in order. **The preview first**, because the whole claim
    // of this milestone is that the authority is legible before it moves: the row names the
    // directory the grant is over and the sentence says what `-r` would have added, so a reader can
    // see the narrower of the two capabilities being chosen.
    line(
        0,
        "caps rm rmtree/rm-solo",
        &[
            "dir      /rmtree  (the directory holding rm-solo)",
            "and nothing under it: no -r, so it cannot even look",
        ],
    ),
    // **The removal, through a caretaker the progenitor built for this one command.** `-v` because `rm`'s
    // default is silence and a gate needs something to read; the name it prints is the name the
    // command line designated, which is the whole of the endowment.
    line(1, "rm -v rmtree/rm-solo", &["rm-solo"]),
    // **And exactly that name went.** Two entries left in a directory that had three, so the grant
    // took what was designated and not what it could reach: `rm-keep` and the whole `rm-doomed`
    // subtree were inside the same capability and are still there, because nothing named them.
    // Eleven bytes of `rm-doomed/` and eight of `rm-keep`, newlines included.
    line(1, "ls rmtree | wc", &["2 2 19"]),
    // **The one shape this still cannot deliver, and the refusal now says what is true.** It used to
    // read "needs the progenitor to build the caretaker", which stopped being true on the line above. A
    // caretaker's whole attenuation is one `OPENDIR` *into* the granted directory, and a name typed
    // at the top prompt designates the root of this shell's namespace, which has no name to descend
    // into; the contract has no verb for "the directory I already hold, with fewer rights". So this
    // is a refusal at the prompt with **nothing spawned**, which is the one outcome this model must
    // never trade away, and it is a design fork rather than a missing line of code. See
    // design/roadmap/31-capability-shell.md and notes/dir-capability.md's BUGS.
    line(0, "rm gate.txt", &["there is no name here to descend into"]),
    // **`xargs`, at the one interface a human touches** (milestone 109). `globmany` holds eleven
    // names one pattern matches, which is more than the eight a single grant can carry.
    //
    // The negative control first, and it is the state of the world this milestone answers: unbatched,
    // a match over the bound is a **refusal at the prompt with nothing spawned**, which is milestone
    // 47's answer at the bound and the reason `xargs` was raised. It still is the answer, because
    // batching is opt-in: a line that silently ran N times would make `caps rm *.txt`'s single
    // printed grant a lie.
    line(
        0,
        "echo globmany/m-*.txt",
        &["matched more names than one grant can carry"],
    ),
    // And batched, the same pattern is swept. **Asserting the second batch is what pins the resume
    // rule**: `m-08.txt` first means batch one ended at `m-07.txt` and the watermark carried, so
    // this one line rules out an off-by-one at the boundary, a batch that restarted from the top,
    // and a batch that took the first eight the directory happened to yield.
    line(
        0,
        "xargs echo globmany/m-*.txt",
        &["batch 2: m-08.txt m-09.txt m-10.txt"],
    ),
    // **And the authority per batch is exactly that batch**, which is the claim the milestone rests
    // on and the one only `caps` can make before the delegation chain exists. The preview prints
    // what `rm` would be handed, and what it would be handed in the second invocation is the three
    // remaining names: not the eleven the pattern matched, and not the directory they live in.
    line(
        0,
        "xargs caps rm globmany/m-*.txt",
        &["the directory holding m-08.txt m-09.txt m-10.txt"],
    ),
    // **Quoting, at the one interface a human touches** (milestone 67). The gap it closes is an
    // authority one: a file called `my notes.txt` could not be *named* before this, so it could not
    // be granted, in a shell whose whole thesis is that naming a resource is granting it.
    //
    // Three lines, and the third is the one that makes the pair a claim. The `>` writes twelve bytes
    // plus a newline into a name only quoting can express; the `<` reads them back; and `wc "my
    // notes.txt"` is the same designation with the operator left out, so if the two disagree, one of
    // them opened something else. That is `gate.txt`'s trio above, asked of a name with a space in
    // it.
    line(0, "echo hello world > \"my notes.txt\"", &[]),
    line(1, "wc < \"my notes.txt\"", &["1 2 12"]),
    line(1, "wc \"my notes.txt\"", &["1 2 12"]),
    // **And the one thing quoting does to authority**: it suppresses expansion, so the same four
    // characters are one name quoted and a set unquoted. `echo` prints what a grant would move, so
    // this line is the narrowing made visible before anything moves. Unquoted, the same pattern is
    // the refusal five lines up.
    line(0, "echo \"*.txt\"", &["*.txt"]),
    // And the preview names it, which is the pairing `caps` exists for: what the line designates is
    // on the screen before anything moves, and a name with a space in it is now something that
    // sentence can be about.
    line(0, "caps wc \"my notes.txt\"", &["input    my notes.txt"]),
    // **Sequencing and the status** (milestone 67). `least_authority_demo 3` runs and `least_authority_demo` alone is refused at
    // the prompt for the integer its manifest requires, so these three lines cover both arms of the
    // condition table with real commands rather than with a branch written for a gate.
    line(1, "least_authority_demo 3 && echo yes", &["yes"]),
    line(0, "least_authority_demo || echo no", &["no"]),
    // **The decision this milestone settled, read at a prompt.** `least_authority_demo` alone is refused, and a
    // refusal is not an error: nothing was spawned, nothing was opened, and the status says so with
    // its own number. Unix cannot draw this line, because there `127` and a program's own `exit(1)`
    // are the same kind of integer.
    //
    // The bare `least_authority_demo` is here because the *first* draft of this gate put `echo $?` straight after
    // `least_authority_demo || echo no` and got `0`, which was the shell being right: the last thing that ran was
    // the `echo`. `$?` is the previous **command**, not the previous line, and that is bash's rule
    // and this shell's.
    line(0, "least_authority_demo", &["needs an integer argument"]),
    line(0, "echo $?", &["2"]),
    // **The progenitor's job budget is bounded and comes back** (milestone 22, the interactive increment).
    // The progenitor now holds a pool with room for six live jobs instead of the kernel's whole construction
    // budget, and every job runs in a region of its own that `job_undertaker` returns when the job ends.
    // **Every job above plus these six go through a six-job pool, several times over**, so a boot
    // where nothing collected would answer "could not spawn (the progenitor is out of memory)"
    // somewhere in here rather than the arithmetic. How many that is was a hand-kept figure in this
    // comment until 2026-09-26 and had drifted; it is now each line's `jobs` tag, summed in the
    // success summary. The `rm` above is the one worth noticing, because it is the first job whose
    // region holds **two** processes, the program and the `fs_subtree_caretaker` carrying its
    // grant, and it is therefore the first thing in this script that would fail if
    // `job_undertaker`'s retry did not collect both. Six distinct arguments rather than one
    // repeated, because the transcript is walked with a moving cursor and six identical answers
    // would let a missed line pass as its neighbour.
    line(1, "least_authority_demo 3", &["3*3 = 9"]),
    line(1, "least_authority_demo 4", &["4*4 = 16"]),
    line(1, "least_authority_demo 5", &["5*5 = 25"]),
    line(1, "least_authority_demo 6", &["6*6 = 36"]),
    line(1, "least_authority_demo 7", &["7*7 = 49"]),
    line(1, "least_authority_demo 8", &["8*8 = 64"]),
    // **The one spawnable program that answers in a register and had no line** until milestone
    // 150's coverage test (`every_spawnable_program_has_a_swish_check_line`) asked. After the six
    // above, so their count is unchanged; the page count it reports depends on page-table overhead,
    // so the assertion is the sentence that says the grant was spent rather than the number.
    line(
        1,
        "memory_grant_depleter --mem 4",
        &["-page budget you granted"],
    ),
    // **The network, from the booted system** (milestone 590 (the booted system starts its network
    // stack)). Every network test
    // before these four started `net_stack` from the kernel's harness; here the progenitor built it
    // at boot from the NIC the kernel granted it, and a program a person typed reaches the runners'
    // echo peer through it. The preview first, because the row it prints is the whole of who may
    // reach the network from this prompt.
    line(
        0,
        "caps network_echo_client --mem 4",
        &[
            "cap 10 endpoint  network  WRITE",
            "a program without this row reaches no network at all",
        ],
    ),
    line(
        1,
        "network_echo_client --mem 4",
        &["echo peer 10.0.2.9:7777 answered: nife-net!"],
    ),
    // **Twice, and the second is the one that says the stack outlives its clients.** The first job
    // handed `net_stack` a page and exited; its region was reclaimed, which revokes that page out
    // of the stack's address space too. A stack that kept a stale window, or a socket number that
    // could not be opened again, answers the first run and fails this one.
    line(
        1,
        "network_echo_client --mem 4",
        &["echo peer 10.0.2.9:7777 answered: nife-net!"],
    ),
    // **The negative control.** A program that declares no network `CALL`s the network's slot
    // anyway; the kernel must refuse it for want of a capability. A spawn service that endowed the
    // stack to every child prints `REACHED` here instead.
    line(
        1,
        "unreachable_network_witness",
        &[
            "network: refused (no capability at slot 10)",
            "entropy: refused (no capability at slot 9)",
            "domain: refused (no capability at slot 7)",
            "slots held: 0\n",
        ],
    ),
    // **A supervised job, interrupted**, under DECISIONS §24 (interrupting the foreground
    // process), and these two are the only lines this gate presses `^C` for (see
    // [`SWISH_CHECK_INTERRUPTED`]). Neither was typed here
    // until 2026-09-25, and in that time the first supervised job of every boot failed with `could
    // not map the job frame`, because the shell mapped its job frame over its own terminal page.
    // The heeder is the cooperative tier: one `^C`, and the job reports it stopped. It goes first
    // because it is the first supervised job of the boot, which is the one that failed.
    line(
        0,
        "interrupt_heeder",
        &[
            "^C interrupts it.",
            "the job caught the interrupt and stopped cleanly after",
        ],
    ),
    // The forcible tier: the ignorer never looks at the flag, so the grace timeout escalates one
    // `^C` to a `DESTROY` of the region the shell built it from, which force-kills its live thread
    // (DECISIONS §16 (object revocation), as amended). A shell whose teardown was refused says so
    // instead.
    line(
        0,
        "interrupt_ignorer",
        &["^C again: tearing the job down.", "its memory reclaimed"],
    ),
    // **A `std` program, spawned by the progenitor rather than by the kernel's test harness**
    // (milestone 595 (provisional)). Until this, every `std` program that ran on nife was built by
    // `kernel/src/user/std_service.rs`, and the progenitor had never produced a child in the layout
    // nife's `std` reads (`crates/std_runtime_protocol`): eight fixed slots, three shared pages,
    // thirty-two stack pages. The preview first, because it is where a person learns the slots
    // moved, and slot 0 is not even the same kind of object as a native child's.
    line(
        0,
        "caps std_exerciser",
        &[
            "cap 0  untyped   heap.",
            "cap 1  endpoint  result   stdout and stderr",
            "cap 5  frame     clock",
            "cap 6  endpoint  entropy  WRITE",
            "cap 7  frame     config",
        ],
    ),
    // **Every phrase is a slot or a page landing where `std` looks for it**, which is why the
    // transcript is the test: the program asserts rather than prints wherever the answer is not
    // deterministic, and panics (a fault this gate fails on) when one is missing. `vec sum` is the
    // heap at slot 0; that anything prints at all is stdout at slot 1; the two `honestly
    // unsupported` lines are slots 4 and 2 left empty, so there is no ambient filesystem or network
    // to fall back on; `wall clock ok` is slot 5 and the page at `CLOCK_PAGE`; `entropy ok` is slot
    // 6; `config seeded` is slot 7 and the page at `CONFIG_PAGE`, read before `main`; and the last
    // line is `process::exit` reaching the supervisor as an exit rather than a fault. The stack is
    // the one thing with no phrase of its own: too little of it is a fault partway through.
    line(
        1,
        "std_exerciser",
        &[
            "hello from std on nife",
            "vec sum 149985000",
            "fs honestly unsupported",
            "net honestly unsupported",
            "wall clock ok",
            "entropy ok",
            "config seeded",
            "exiting through process::exit",
        ],
    ),
    line(0, "echo shell-boot-gate-done", &["shell-boot-gate-done"]),
];

/// **The lines after which this gate presses `^C`**, once the shell has said the job is running.
/// Each runs until interrupted, so without the keystroke the prompt never comes back. A program
/// here is also one `every_spawnable_program_has_a_swish_check_line` would otherwise have to excuse.
const SWISH_CHECK_INTERRUPTED: [&str; 2] = ["interrupt_heeder", "interrupt_ignorer"];

/// What the shell prints once a supervised job is running and watched. Pressing `^C` before it
/// would still be counted (the shell's watermark catches an early one), but after it is the case a
/// person types, so it is the one this gate proves.
const SWISH_CHECK_RUNNING: &str = "^C interrupts it.";

/// **The lines a leg does not type, each with the reason** (milestone 182 (`x86_64`'s own
/// interactive-boot entry point), under the rule
/// milestone 150 added: an omitted line carries a stated reason, or it is a gap nobody can see).
///
/// A function over the line rather than a second table, so a line added to [`SWISH_CHECK_SCRIPT`]
/// is typed on every leg by default and an omission is the thing that has to be argued for. `None`
/// means the line runs. Every omission but one is `x86_64`'s; the one the other two legs make is
/// the disk install that stands in, on `x86_64`, for a fetch they make over the network.
fn swish_check_omits(arch: &str, line: &str) -> Option<&'static str> {
    if arch != "x86_64" {
        return (line == "package install downloads/greeting.nifepkg").then_some(
            "this leg fetched the same package over the network the line before; x86_64 has no \
             NIC, so it installs it from the disk instead",
        );
    }
    // The four `uuid` lines and `std_exerciser` used to be here, for want of an entropy service at
    // the x86_64 prompt: the progenitor built one only from a virtio-rng, and `q35` has no mmio bus
    // to find one on. Milestone 595 (provisional) gave the progenitor the kernel's service on
    // `RDSEED` instead (`kernel::user::boot_instruction_entropy`), so they run.
    match line {
        // The kernel grants the progenitor a NIC only from a virtio-mmio slot, and the x86_64
        // runner attaches no
        // `-netdev` at all until milestone 494 (a driver for the network card a PC actually has).
        // The preview and the witness stay: neither needs a device, and the witness's refusal is
        // the same on a boot with no stack as on one that has a stack and did not endow it.
        // And the package source is reached over that network (milestone 198 rung 3a's fetch).
        // `package install nosuch` stays: the catalogue refuses it before the network is asked.
        "network_echo_client --mem 4" | "package install uptime" | "package install greeting" => {
            Some(
                "x86_64 has no NIC the progenitor can build a network stack from (virtio-net is \
                 found on virtio-mmio only, and the x86_64 runner attaches none)",
            )
        }
        _ => None,
    }
}

/// The first thing `x86_hand_over` prints (`kernel/src/main.rs`), where the `x86_64` leg starts
/// reading for faults; see `after_hand_over` in [`swish_check_leg`]. The same sentence
/// `uefi_boot` requires.
const X86_HAND_OVER_START: &str = "nife: handing the system to the userspace progenitor.";

/// The last thing `x86_hand_over` prints (`kernel/src/main.rs`) once the progenitor has outlived
/// its ten-second watch, which is the ordinary interactive outcome. The `x86_64` leg waits for it
/// before typing; see [`swish_check_leg`]'s doc.
const X86_HAND_OVER_REPORT: &str = "as a port capability (milestone 299).";

/// How long to wait for the banner, for one line's echo, and for the whole transcript. Generous:
/// under TCG on a loaded machine a cold boot to the prompt is seconds, and a gate that flakes on a
/// busy laptop is a gate people learn to ignore.
///
/// # BUGS
///
/// **It flakes anyway, and the thirty seconds is a *per-echo* budget rather than a per-line one.**
/// On 2026-08-18 the riscv64 leg failed with "the prompt never echoed `caps date 2> err.txt`" after
/// echoing thirteen characters of it, on a machine running four other lanes; the same commit passed
/// on a rerun with nothing else changed. So a failure of this shape is a load report and not a
/// finding, and the way to tell them apart is to rerun the one leg before reading the transcript.
/// Raising the number is not obviously the fix: a real hang would then take proportionally longer
/// to report, and the honest measurement (how long an echo actually takes under load, versus the
/// budget) has not been made.
const SWISH_CHECK_BOOT_SECS: u64 = 120;
const SWISH_CHECK_LINE_SECS: u64 = 30;

/// **The `x86_64` leg's per-line bound, three times the others', and measured rather than chosen**
/// (milestone 182, 2026-09-19).
///
/// Under OVMF the console server hands every write to the screen terminal and waits for it to be
/// drawn (milestone 400), so a line costs what its output costs to paint and copy, not what the
/// shell costs to run it. Measured on patagonia, one run each, typed to prompt-back:
///
/// | leg | 60 or 64 lines | slowest line |
/// |---|---|---|
/// | `aarch64` | 6.9 s | `apropos capability` 0.3 s |
/// | `riscv64` | 7.3 s | `apropos capability` 0.6 s |
/// | `x86_64` | 321.1 s | `xargs caps rm globmany/m-*.txt` 24.7 s, then `caps ps` 16.7 s |
///
/// CI's runner was 1.5x to 1.8x slower than patagonia on this leg (run 35463884897: the guest's
/// `date` ran 119 s into the leg against 80 s here, and `caps ps`, 16.7 s here, did not finish in
/// 30 s there), which is how the 30 s bound went red on a line that has no defect. 90 s is 3.6x
/// the slowest local line and 2x that line at CI's worst measured ratio. A real hang still fails,
/// ninety seconds later than it would elsewhere.
///
/// **Which part is the emulator's.** The same shell over TCG answers every line in under a second
/// on the other two legs, so the whole difference is the screen path: `display_terminal` paints
/// the damaged cells (the whole 924x344 surface on every scroll) and `framebuffer_driver` copies
/// them into an uncacheable aperture one word at a time, both as unoptimised debug builds, each
/// store through TCG. A real PC pays the same copy in native stores at uncacheable speed, which is
/// milliseconds per scroll rather than seconds and is not measured on silicon
/// (`framebuffer_driver`'s BUGS). Milestone 400's BUGS records the design half: the console
/// blocks on the screen.
const SWISH_CHECK_X86_LINE_SECS: u64 = 90;

/// How many foreign characters [`find_marker`] will step over inside one marker before it gives up.
///
/// The intruder is one kernel fault report, three lines and about 150 characters. 400 is that with
/// room to spare. It is a ceiling and not the thing doing the work: what makes this safe is the
/// **order** the checks run in ([`boot_claim`]), not how generous this number is.
const SWISH_CHECK_MARKER_SLACK: usize = 400;

/// Text the **kernel** prints only in a user-fault report, which is the only thing it writes after
/// the userspace console has started.
///
/// Six of them, deliberately, and short ones. This is the test for "was a second writer active
/// while the progenitor was printing", and the honest thing to say about it is that any single string can
/// itself be shuffled apart: milestone 230's second CI failure destroyed `the kernel is fine.` into
/// `the kernel iis fnit: constiner.`. Six independent chances is not a proof, it is a much better
/// bet than one, and what happens when they all lose is a loud failure rather than a silent pass.
///
/// **The first two are a pair and are used as one**, by the killed-thread check below as well as by
/// [`kernel_wrote_during_boot`]: `user thread ` alone appears in ordinary prose and ` killed: ` is
/// the half that says a fault report. Both, in order, are the aarch64 and riscv64 fault printer's
/// own first line (`kernel/src/arch/*/exceptions.rs`). One array rather than two so the two uses
/// cannot drift apart, which is what milestone 231's own comment asked for when it had its own copy.
const KERNEL_FAULT_TOKENS: [&str; 6] = [
    "user thread ",
    " killed: ",
    "the kernel is fine",
    "stval 0x",
    "esr 0x",
    " sp 0x",
];

/// What `kernel::cap::report_peak` prints, verbatim (milestone 231). The line carries the boot's
/// capability-slot high-water mark against the table's capacity, and `swish-check` both echoes the
/// last one it sees and fails if the kernel flagged it as past the recorded peak.
const SLOT_GAUGE: &str = "capability slots:";

/// Where a marker was found in a transcript, and what it cost to find it.
enum Marker<'a> {
    /// Present, contiguous. What a boot with one writer gives.
    Exact,
    /// The marker's characters are all present, in order, with someone else's bytes wedged between
    /// them. Carries the intruding text so a caller can name it rather than hide it.
    Interleaved(String),
    /// Not there. Carries the longest prefix of the marker that appears contiguously, which is the
    /// most useful single fact about a transcript that does not have it.
    Absent { matched: &'a str },
}

/// Find `needle` in `haystack`, tolerating another writer's bytes wedged into the middle of it.
///
/// **This is not a safety mechanism and must not be used as one.** It answers "could these
/// characters be this marker, shuffled?", which is a question with false positives by construction:
/// a transcript containing `budget NOT dropped` answers yes for `budget dropped` at a cost of four
/// skipped characters. [`boot_claim`] is what makes it safe, by asking the un-shuffle-able question
/// first. Two earlier versions of this tried to carry the safety themselves, by a character budget
/// and then by requiring the kernel's own signature inside the skipped text, and a shuffle worse
/// than the fixture defeated each in turn. The third attempt is not this function; it is not asking
/// this function to decide.
fn find_marker<'a>(haystack: &str, needle: &'a str) -> Marker<'a> {
    if haystack.contains(needle) {
        return Marker::Exact;
    }
    let text: Vec<char> = haystack.chars().collect();
    let want: Vec<char> = needle.chars().collect();
    let mut best: Option<String> = None;
    for start in 0..text.len() {
        if text[start] != want[0] {
            continue;
        }
        let mut i = 1usize;
        let mut j = start + 1;
        let mut skipped = String::new();
        while i < want.len()
            && j < text.len()
            && skipped.chars().count() <= SWISH_CHECK_MARKER_SLACK
        {
            if text[j] == want[i] {
                i += 1;
            } else {
                skipped.push(text[j]);
            }
            j += 1;
        }
        if i == want.len() && best.as_ref().is_none_or(|b| skipped.len() < b.len()) {
            best = Some(skipped);
        }
    }
    match best {
        Some(skipped) => Marker::Interleaved(skipped),
        None => {
            let mut matched = 0usize;
            for end in (1..=needle.len()).rev() {
                if needle.is_char_boundary(end) && haystack.contains(&needle[..end]) {
                    matched = end;
                    break;
                }
            }
            Marker::Absent {
                matched: &needle[..matched],
            }
        }
    }
}

/// Was the kernel writing the UART **while the progenitor was printing**?
///
/// Only the boot phase counts, which is everything before the shell's banner: a fault after the
/// prompt is out cannot explain a boot line that was already read. The kernel's boot tour is
/// deliberately not among the tokens, because it prints on every boot and a test that is always
/// true would make [`BootClaim::Unreadable`] a way to pass without evidence.
fn kernel_wrote_during_boot(transcript: &str) -> bool {
    let boot = transcript
        .find("nife capability shell")
        .map_or(transcript, |at| &transcript[..at]);
    KERNEL_FAULT_TOKENS.iter().any(|t| boot.contains(t))
}

/// What a transcript says about one thing the progenitor reports on itself.
enum BootClaim {
    /// The progenitor said the thing that is true. Carries the intruding text when it had to be un-shuffled.
    Affirmed(Option<String>),
    /// **The progenitor said the opposite**, contiguously. The failing answer, and the one with teeth.
    Denied,
    /// Neither sentence is readable, and the kernel was writing over the progenitor while it printed. Not a
    /// failure: this check cannot see through a shuffle and should not pretend it can.
    Unreadable { longest_run: String },
    /// Neither sentence is there and nothing else was writing, so nothing shuffled it. The progenitor did not
    /// say this at all, which is a real failure and the one that keeps this check from passing
    /// against a boot that stopped reporting.
    Silent,
}

/// Read one of the progenitor's claims about itself out of a transcript that **two processes wrote at once**.
///
/// # Why this is shaped the way it is
///
/// The kernel prints fault reports with its own UART driver and the userspace `console` server
/// drives the same device from another address space, with nothing arbitrating between them. The
/// result is not truncation, it is a **byte-level shuffle**, and it is nondeterministic: milestone
/// 230 saw the same code pass one CI run and fail the next, and saw `construction budget dropped`
/// reduced to a longest surviving run of `const`. **No matcher can be made reliable against that**,
/// because any string a matcher keys on can itself be split, including the kernel's own signature
/// (`the kernel is fine.` came out as `the kernel iis fnit: constiner.`).
///
/// So the safety does not live in the matching. It lives in **which question is asked first**, and
/// in one asymmetry that a shuffle cannot break:
///
/// > Interleaving can **destroy** a string. It cannot **create** one.
///
/// Therefore an exact search for the sentence the progenitor prints when the answer is *no* has no false
/// positives: if `construction budget NOT dropped` is in the transcript, the progenitor printed it. That is
/// the check with the teeth, it runs first, and it is exact rather than tolerant precisely so that
/// nothing shuffled can be mistaken for it.
///
/// Everything after it only decides between passing and saying why:
///
/// 1. `negative` present, **exactly** -> [`BootClaim::Denied`]. The progenitor reported the failing answer.
/// 2. `positive` present, exactly or shuffled -> [`BootClaim::Affirmed`].
/// 3. Neither, and the kernel was writing during the boot -> [`BootClaim::Unreadable`]. A pass, and
///    the caller says so out loud.
/// 4. Neither, and nothing else was writing -> [`BootClaim::Silent`]. A failure: with one writer
///    there is nothing to shuffle, so the progenitor really did not say it.
///
/// # What this trades, said plainly
///
/// It moves the residual error from **false red to false green**, on purpose, and that is the right
/// direction for a check that runs in CI on every lane. A false red taxes work that is not the cause
/// and this tree has deleted three checks for that signature. A false green here is recoverable by
/// repetition, because the failure it guards is persistent rather than transient: a progenitor that stops
/// dropping its budget prints the negative sentence on *every* boot, on both legs, on every push, so
/// hiding it requires the shuffle to land on that sentence every time.
///
/// The residual hole is case 4's converse and is named in `script/swish-check`'s own BUGS: if the progenitor's
/// report were deleted **and** a thread faulted in the same boot, this passes. Both halves have to
/// happen together, and the second is itself a defect the transcript shows.
fn boot_claim(transcript: &str, positive: &str, negative: &str) -> BootClaim {
    if transcript.contains(negative) {
        return BootClaim::Denied;
    }
    match find_marker(transcript, positive) {
        Marker::Exact => BootClaim::Affirmed(None),
        Marker::Interleaved(skipped) => BootClaim::Affirmed(Some(skipped)),
        Marker::Absent { matched } => {
            if kernel_wrote_during_boot(transcript) {
                BootClaim::Unreadable {
                    longest_run: matched.to_string(),
                }
            } else {
                BootClaim::Silent
            }
        }
    }
}

/// Read the transcript as it stands right now.
///
/// A named function rather than `seen.lock().expect(..).clone()` written inline at each check,
/// because the lock is held for the length of the expression and a check that also wants to format
/// the transcript into a message would otherwise hold it while doing so.
fn transcript_now(seen: &std::sync::Arc<std::sync::Mutex<String>>) -> String {
    seen.lock().expect("transcript lock").clone()
}

/// Check one of the progenitor's claims and turn it into a complaint, or `None` if it passed.
///
/// The two passing outcomes both print to stderr when they were not clean, because a transcript
/// that needed un-shuffling is evidence of the UART defect and swallowing it would hide the thing
/// this whole mechanism exists because of.
///
/// `subject` is what the progenitor reports on, in words a reader can act on. It describes the **program's**
/// behaviour, which this function can honestly assert. It never describes the machine's state,
/// which it cannot: the diagnostic this replaced said a missing string meant the progenitor "still holds the
/// kernel's root untyped, or the delete did not take", about a boot where the progenitor had dropped the
/// budget and said so, and sent a maintainer hunting a capability bug that does not exist.
fn boot_claim_complaint(
    transcript: &str,
    subject: &str,
    positive: &str,
    negative: &str,
) -> Option<String> {
    match boot_claim(transcript, positive, negative) {
        BootClaim::Affirmed(None) => None,
        BootClaim::Affirmed(Some(skipped)) => {
            eprintln!(
                "swish-check: read {positive:?} only after stepping over {} characters another \
                 writer had spliced through it. Every byte is present and in order, so this is the \
                 kernel's fault printer and the userspace console sharing the UART, not a lost \
                 read. The intruding text was {skipped:?}",
                skipped.chars().count(),
            );
            None
        }
        BootClaim::Unreadable { longest_run } => {
            eprintln!(
                "swish-check: could not read the progenitor's report on {subject}. Neither sentence survives \
                 in the transcript (the longest run of the affirmative one that does is \
                 {longest_run:?}), AND the kernel printed a fault report during the boot, so two \
                 processes were writing the UART at once and the line cannot be recovered. NOT \
                 treated as a failure, because this check cannot see through a byte-level shuffle. \
                 A real regression prints the negative sentence on every boot and is caught by the \
                 exact search for it above.",
            );
            None
        }
        BootClaim::Denied => Some(format!(
            "The progenitor reported the failing answer on {subject}: the transcript contains {negative:?}, \
             contiguously. Interleaving can destroy a string and cannot create one, so the progenitor printed \
             this."
        )),
        BootClaim::Silent => Some(format!(
            "the transcript carries neither of the progenitor's two sentences about {subject} ({positive:?} \
             nor {negative:?}), and nothing else was writing the UART during the boot, so nothing \
             shuffled them. The progenitor did not report this at all. That is what this check knows; it \
             reads strings out of a transcript and asserts nothing about the kernel. The full \
             transcript is below."
        )),
    }
}

/// One architecture's leg of [`swish_check`]: `aarch64`, `riscv64` or `x86_64`.
///
/// # The `x86_64` leg boots under firmware (milestone 182)
///
/// **Under OVMF, from the same `\EFI\BOOT\BOOTX64.EFI` `cargo xtask uefi-image` stages for a
/// USB stick**, not through QEMU's PVH `-kernel` loader. That image is the thing a customer boots
/// (DECISIONS §157, milestone 198's rung 1), so it is the one worth typing at: the loader, the
/// firmware's memory map and ACPI tables, and the console server's tee onto the firmware's screen
/// (milestone 400) are all on the path, and the PVH boot has none of them. What it costs over PVH
/// is recorded in milestone 182's block, measured rather than asserted.
///
/// Three `x86_64` differences, each forced by the machine rather than chosen:
///
/// - **The default kernel, not `--features shell`.** `x86_64` has no early hand-over: every boot
///   runs the tour and then hands over (milestone 268), and `uefi_image` builds exactly that.
/// - **The kernel's hand-over report lands after the prompt.** `x86_hand_over` watches the
///   progenitor for ten seconds and then prints two lines, so the transcript does not end in `$ `
///   until something is typed. The leg waits for that report and then presses Enter once, so the
///   report cannot splice into a typed line's echo and the first line meets a fresh prompt.
/// - **No virtio-rng, so `RDSEED`.** Every entropy device the kernel can find is virtio-mmio and
///   `q35` has no mmio bus, so the kernel builds the progenitor's entropy service on the CPU's seed
///   instruction instead (milestone 595 (provisional)), which the runner's `-cpu max` implements.
///   The leg asserts the progenitor said so, because a boot that silently fell back to no entropy
///   would otherwise surface only as the `uuid` lines failing.
fn swish_check_leg(arch: &str) -> bool {
    swish_check_boot(arch, SWISH_CHECK_SCRIPT, true)
        && swish_check_boot(arch, SWISH_CHECK_AFTER_REBOOT, false)
}

/// **One boot of [`swish_check_leg`]**: build (when `fresh`), boot, type `script`, read the answers.
/// `fresh` is false for the second boot, which runs against the disk the first one left behind and
/// builds nothing, because what it proves is that the disk is the only thing carried across
/// (milestone 198 (a package manager) rung 3a: an installed package survives a reboot).
fn swish_check_boot(arch: &str, script: &[Line], fresh: bool) -> bool {
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    let riscv = arch == "riscv64";
    let x86 = arch == "x86_64";
    // **`std_exerciser` is in this boot's archive only if it was built** (milestone 595
    // (provisional)): `cargo xtask std-exerciser` compiles it against the `nife-dev` toolchain, which
    // `script/test` runs and a bare `script/swish-check` does not. Without it the progenitor has no
    // image and the line would answer "could not spawn", which is a fact about this checkout rather
    // than the boot, so the line is skipped and says why. **Not in CI**, where `test` always builds
    // it first: a missing image there means the build broke, and skipping would hide exactly that.
    let std_built = crate::farm::std_exerciser_elf(&format!("{arch}-unknown-nife")).exists();
    // Said once per leg, on the first boot: the second boot types no `std` line.
    if fresh && !std_built {
        if std::env::var_os("CI").is_some() {
            eprintln!(
                "swish-check ({arch}): no std_exerciser was built for this architecture, and in CI \
                 `test` builds it first; refusing to skip its line"
            );
            return false;
        }
        eprintln!(
            "swish-check ({arch}): skipping `std_exerciser`: it is not built here (`cargo xtask \
             std-exerciser` builds it; `script/test` runs that)"
        );
    }
    let skipped = |line: &str| {
        swish_check_omits(arch, line).is_some() || (line == "std_exerciser" && !std_built)
    };
    eprintln!();
    eprintln!(
        "--- swish-check ({arch}): boot {} and type at the prompt ---",
        if x86 {
            "the UEFI image under OVMF"
        } else {
            "`--features shell`"
        }
    );

    // The same build the interactive boot takes, because a gate that builds something else is
    // gating something else. The FS server first (`user()` packs the initrd by reading the ELF off
    // disk), then the RedoxFS image, because the runner attaches the disk only when the file is
    // there and `<` and `>` need one.
    let target = if riscv { RISCV_TARGET } else { TARGET };
    let built = !fresh
        || if x86 {
        // `uefi_image` packs the archive, builds the kernel against it, and stages the loader;
        // the FS server has to exist first so the archive carries it.
        redoxfs_server_build(X86_TARGET) && mkdisk() && mkredoxfs() && uefi_image()
    } else if riscv {
        redoxfs_server_build(RISCV_TARGET) && mkdisk() && mkredoxfs() && initrd_riscv()
    } else {
        redoxfs_server_build(TARGET) && mkredoxfs() && mkdisk() && user()
    } // After the archive build, whose packages it copies: a package the image's catalogue
    // vouches for, a tampered copy, and an unvouched program, on the disk for the installer's
    // lines in the script (milestone 198 rung 3a).
    && crate::disk::seed_installed(arch)
    && (x86
        || run(
            "cargo",
            &[
                "build",
                "-p",
                "kernel",
                "--features",
                "shell",
                "--target",
                target,
            ],
        ));
    if !built {
        return false;
    }

    // The runner directly rather than through `cargo run`, so the process this owns **is** QEMU
    // (the runner script `exec`s it). A `cargo run` in between would leave the emulator alive when
    // the kill lands on cargo, which is the leak CLAUDE.md's QEMU rule exists about.
    let mut cmd = if x86 {
        // OVMF, one core (the runner's default, for `ap_boot`'s BUGS #3). The runner bounds itself
        // with `qemu-bounded.sh`; the bound here is every wait below added up, so it only ever
        // fires on a leg that has already failed.
        let mut c = Command::new("helpers/qemu-uefi-x86_64.sh");
        c.arg(esp_dir());
        c.env(
            "NIFE_UEFI_TIMEOUT",
            (SWISH_CHECK_BOOT_SECS * 2 + SWISH_CHECK_X86_LINE_SECS * (script.len() as u64 + 2))
                .to_string(),
        );
        c.env_remove("NIFE_NVME");
        // The RedoxFS disk, which `>`, `<`, `ls` and `rm` need; opt-in on this runner, and its
        // header says why.
        c.env("NIFE_UEFI_REDOXFS", "1");
        c
    } else {
        let mut c = Command::new(if riscv {
            "helpers/qemu-runner-riscv64.sh"
        } else {
            RUNNER
        });
        c.arg(format!("target/{target}/{}/kernel", profile_dir()));
        c.env(
            "NIFE_INITRD",
            if riscv {
                riscv_initrd_path()
            } else {
                initrd_path()
            },
        );
        c
    };
    cmd.env("NIFE_DISK", disk_path());
    // A virtio-rng device (DECISIONS §120's 2026-08-26 amendment: "grant the QEMU-only virtio-rng
    // stopgap"), unlike the GPU/keyboard/NVMe flags above `test()` sets: this is the interactive
    // boot itself, not the bench boot sharing its runner, so there is no icount-drift reason to
    // keep it test-leg only, and the whole point of the amendment is that this boot should have
    // one. `cmd.env`, not `test()`'s own `std::env::set_var`, because this function builds its own
    // `Command` directly (see `NIFE_INITRD`/`NIFE_DISK` just above) rather than spawning through
    // the global-env-inheriting path `test()` uses.
    cmd.env("NIFE_RNG", "1");
    // **And a NIC** (milestone 590 (provisional)), on the two legs whose runner can attach one:
    // the progenitor builds `net_stack` from it, and `network_echo_client` reaches the runners'
    // echo peer through that. The runner attaches two (an mmio NIC and a PCIe one behind the
    // IOMMU); the kernel grants the progenitor the mmio one and the other sits unclaimed.
    if !x86 {
        cmd.env("NIFE_NET", "1");
        // **What the package source serves this leg** (milestone 198 rung 3a's fetch). The runner
        // starts `helpers/package-http-peer` once per connection, and it inherits this through
        // QEMU; `disk::stage_installed` filled the directory.
        cmd.env(
            "NIFE_PACKAGE_SOURCE",
            crate::disk::package_source_dir(arch).display().to_string(),
        );
    }
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("swish-check: failed to start the runner: {e}");
            return false;
        }
    };
    let mut stdin = child.stdin.take().expect("piped stdin");
    let mut stdout = child.stdout.take().expect("piped stdout");

    // A reader thread rather than blocking reads on this one, because every wait below needs a
    // deadline: a boot that hangs is exactly the failure this gate is for, and a gate that hangs
    // with it reports nothing.
    let seen = Arc::new(Mutex::new(String::new()));
    let collector = Arc::clone(&seen);
    let reader = std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        while let Ok(n) = stdout.read(&mut buf) {
            if n == 0 {
                return;
            }
            // The terminal's own carriage returns are the line editor's, not content. Dropped so
            // the checks below can be about what a person reads.
            let text = String::from_utf8_lossy(&buf[..n]).replace('\r', "");
            collector.lock().expect("transcript lock").push_str(&text);
        }
    });

    // Poll for a needle **after `from`** with a deadline, `from` being how long the transcript was
    // when the thing we are waiting for was asked for.
    //
    // The position matters because this gate deliberately types `wc < gate.txt` twice. A
    // whole-transcript search finds the first one's echo instantly and the wait returns before the
    // second line has been read at all, which then types the line after it into a prompt that has
    // not appeared. That is exactly what the first version of this did.
    let wait_after = |from: usize, needle: &str, secs: u64| -> bool {
        let deadline = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < deadline {
            if seen.lock().expect("transcript lock")[from..].contains(needle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    };
    let mark = || seen.lock().expect("transcript lock").len();

    // **Wait for the prompt to come back before typing the next line**, and this is not politeness.
    // The line editor echoes a character the moment it arrives, whether or not the shell has asked
    // for a line yet, so typing ahead produces a transcript in which a command's echo appears
    // *before* the `$ ` that should introduce it. The first version of this gate typed ahead and
    // then failed to find its own echo, which is a bug in the gate rather than in the shell.
    //
    // A transcript ending in the bare prompt is the unambiguous "ready": the prompt is out and
    // nothing has been echoed since.
    let wait_for_prompt = |secs: u64| -> bool {
        let deadline = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < deadline {
            if seen.lock().expect("transcript lock").ends_with("$ ") {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    };

    // **What the checks below read, which on x86_64 starts at the hand-over.** The x86_64 kernel
    // runs its tour before handing over (milestone 268), and the tour's userspace demonstration
    // kills two threads on purpose (`x86_userspace_demo`'s supervised deaths, reported as "died
    // at pc ..., delivered to its supervisor"). Those are the kernel's fault path working, printed
    // before any process this gate is about exists, so the fault check and the "was the kernel
    // writing during the boot" test both start where the progenitor does. The other two legs boot
    // `--features shell`, which has no tour, so for them this is the whole transcript.
    let after_hand_over = |t: &str| -> String {
        if x86 {
            t.find(X86_HAND_OVER_START)
                .map_or(t, |at| &t[at..])
                .to_string()
        } else {
            t.to_string()
        }
    };

    // Everything below must reach the kill, so failures are recorded rather than returned.
    let mut failed: Vec<String> = Vec::new();
    // The banner is the first claim: the progenitor built the console, the line editor, the input driver and
    // the shell, and gave the shell every capability it needs to say hello. A boot that dies in any
    // of that prints nothing, which is the symptom all three of this milestone's bugs shared.
    if !wait_after(0, "nife capability shell", SWISH_CHECK_BOOT_SECS) {
        failed.push(format!(
            "no prompt banner within {SWISH_CHECK_BOOT_SECS}s: the `--features shell` boot never \
             reached a shell"
        ));
    } else {
        // **The progenitor gave the construction budget away, and says so from the inside** (milestone 22,
        // the interactive increment). The progenitor prints this one line after deleting the root untyped and
        // before starting the shell, and it prints it only when `RETYPE` and `RETYPE_OBJ` on that
        // slot both answered `NoSuchSlot`: the capability is gone, not narrowed. The other branch
        // says "NOT dropped", so a boot that kept its budget fails here rather than passing quietly.
        // It is already in the transcript by now, because the banner comes from a shell the progenitor starts
        // afterwards; there is nothing to wait for.
        //
        // **The message says what was not found, and nothing about the kernel.** It used to name
        // two capability states ("it still holds the root untyped, or the delete did not take"),
        // which are the two reasons the progenitor would print the other branch, and which this check has no
        // evidence for: all it ever knows is whether a string is in a transcript. On milestone
        // 230's first CI run it said exactly that about a boot where the progenitor had dropped the budget
        // and had said so, and sent a maintainer looking for a capability bug that does not exist.
        // A missing marker means a missing marker. The transcript is printed below; that is the
        // evidence, and this line's job is to say which string was wanted and how close it came.
        if let Some(complaint) = boot_claim_complaint(
            &after_hand_over(&transcript_now(&seen)),
            "giving the construction budget away",
            "construction budget dropped; retype answers NoSuchSlot",
            "construction budget NOT dropped",
        ) {
            failed.push(complaint);
        }
        // **And the progenitor measured every program it loaded** (milestone 104), which is the line that
        // keeps the second link of the chain from evaporating. A kernel built without the
        // measurement step refuses to boot at all, but a *table* that stopped naming things would
        // leave a system that boots, prompts, and vouches for nothing, and it would look exactly
        // like a healthy one. So the progenitor says which way it went either way, and the affirmative
        // sentence is what this gate reads. The other branch names the programs it refused, so a
        // boot that quietly stopped spawning half the prompt's commands fails here.
        if let Some(complaint) = boot_claim_complaint(
            &after_hand_over(&transcript_now(&seen)),
            "measuring the programs it loads",
            "every program measured against the archive table",
            "measurement refused",
        ) {
            failed.push(complaint);
        }
        // **And the progenitor has an entropy service, from the source this leg's machine has**
        // (milestone 595 (provisional)). The `uuid` and `std_exerciser` lines below would fail
        // without one, but as a refused draw several lines on, which says nothing about why. This
        // names the source: a virtio-rng on the two `virt` machines, the CPU's `RDSEED` on `q35`,
        // where the kernel builds the service and the progenitor is granted it. The negative is
        // the kernel's own refusal, which both of its no-entropy sentences end with.
        if let Some(complaint) = boot_claim_complaint(
            &after_hand_over(&transcript_now(&seen)),
            "its entropy source",
            if x86 {
                "entropy service up; the kernel built it on the CPU's seed instruction"
            } else {
                "entropy service up; drew real bytes from a virtio-rng device"
            },
            "nothing at the prompt can draw random bytes",
        ) {
            failed.push(complaint);
        }
        // **x86_64: let the kernel finish its hand-over report first**, then press Enter for a
        // fresh prompt (this function's doc says why). The report is the boot thread's last
        // output, so after it the shell is the UART's only writer until something faults.
        let mut ready = true;
        if x86 {
            ready = false;
            if !wait_after(0, X86_HAND_OVER_REPORT, SWISH_CHECK_BOOT_SECS) {
                failed.push(format!(
                    "the kernel never printed its hand-over report ({X86_HAND_OVER_REPORT:?}), so \
                     the progenitor did not outlive `x86_hand_over`'s watch"
                ));
            } else if writeln!(stdin).is_err() || stdin.flush().is_err() {
                failed.push("could not press Enter at the prompt".to_string());
            } else {
                ready = true;
            }
        }
        // **How long each line took**, typed to prompt-back, so every run reports its own margin
        // against the per-line bound rather than leaving it to be guessed after a red one.
        let line_secs = if x86 {
            SWISH_CHECK_X86_LINE_SECS
        } else {
            SWISH_CHECK_LINE_SECS
        };
        let mut took: Vec<(&str, Duration)> = Vec::new();
        let mut previous: Option<(&str, Instant)> = None;
        for &Line { typed: line, .. } in script {
            if !ready {
                break;
            }
            if skipped(line) {
                continue;
            }
            if !wait_for_prompt(line_secs) {
                failed.push(format!(
                    "the prompt never came back to take `{line}`; the line before it did not finish"
                ));
                break;
            }
            if let Some((prev, typed)) = previous {
                took.push((prev, typed.elapsed()));
            }
            previous = Some((line, Instant::now()));
            let at = mark();
            if writeln!(stdin, "{line}").is_err() || stdin.flush().is_err() {
                failed.push(format!("could not type `{line}` at the prompt"));
                break;
            }
            if !wait_after(at, &format!("{line}\n"), line_secs) {
                failed.push(format!("the prompt never echoed `{line}`"));
                break;
            }
            if SWISH_CHECK_INTERRUPTED.contains(&line) {
                if !wait_after(at, SWISH_CHECK_RUNNING, line_secs) {
                    failed.push(format!(
                        "`{line}` never said it was running under supervision ({SWISH_CHECK_RUNNING:?})"
                    ));
                    break;
                }
                // ETX, the byte a terminal sends for `^C`, with no newline: the line editor acts on
                // it the moment it arrives.
                if stdin.write_all(&[0x03]).is_err() || stdin.flush().is_err() {
                    failed.push(format!("could not press ^C under `{line}`"));
                    break;
                }
            }
        }
        // One more, for the last line: every other answer is bounded by the next line's wait, and
        // the last one has no next line. Without this the transcript is read while the final
        // command is still running.
        if failed.is_empty() && !wait_for_prompt(line_secs) {
            failed.push("the prompt never came back after the last line".to_string());
        }
        if let (true, Some((prev, typed))) = (failed.is_empty(), previous) {
            took.push((prev, typed.elapsed()));
        }
        // Every line's time, in script order, beside the transcript when that was asked for.
        if std::env::var_os("NIFE_SHOW_TRANSCRIPT").is_some() {
            for (l, d) in &took {
                eprintln!("swish-check ({arch}): {:6.2}s  {l}", d.as_secs_f64());
            }
        }
        took.sort_by_key(|t| std::cmp::Reverse(t.1));
        let total: Duration = took.iter().map(|(_, d)| *d).sum();
        eprintln!(
            "swish-check ({arch}): {} lines in {:.1}s; slowest, against a {line_secs}s \
             bound per line: {}",
            took.len(),
            total.as_secs_f64(),
            took.iter()
                .take(3)
                .map(|(l, d)| format!("`{l}` {:.1}s", d.as_secs_f64()))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    let whole = seen.lock().expect("transcript lock").clone();
    let transcript = after_hand_over(&whole);
    // The transcript is printed on failure below, because that is when somebody needs it. This
    // prints it on success too, and it exists because the notes in this tree quote real prompt
    // sessions: `NIFE_SHOW_TRANSCRIPT=1 script/swish-check --arch aarch64` is where the EXAMPLES
    // in notes/swish-language.md and notes/pipes.md come from, rather than from somebody retyping
    // what they remember the shell saying.
    if std::env::var_os("NIFE_SHOW_TRANSCRIPT").is_some() {
        eprintln!("--- swish-check ({arch}) transcript ---");
        eprintln!("{whole}");
    }
    if failed.is_empty() {
        // Walked in order with a moving cursor, not searched. The script types `wc < gate.txt`
        // twice on purpose and the two answers are the whole point of the append arm, so a search
        // that found either one would read the same answer for both lines and pass a `>>` that had
        // truncated.
        let mut cursor = 0usize;
        for &Line {
            typed: line,
            jobs,
            answer: want,
        } in script
        {
            if skipped(line) {
                continue;
            }
            match swish_check_answer(&transcript, cursor, line) {
                Some((answer, next)) => {
                    cursor = next;
                    // **Every wanted phrase, not the first**, because one answer can carry several
                    // independent claims and checking one of them makes the rest decoration. `caps`
                    // is the case that forced it: it prints the shell's whole endowment, and a gate
                    // that read only the clock row would pass a boot that had stopped granting the
                    // shell a directory, which is the wiring milestone 31's headline rests on.
                    for want in want {
                        if !answer.contains(want) {
                            failed.push(format!(
                                "`{line}` answered {:?}, wanted {want:?}",
                                answer.trim()
                            ));
                        }
                    }
                    // **And a line that runs jobs ran them** ([`Line`]'s doc): the summary's job
                    // count is summed from these tags, so a tagged line whose job was never built
                    // must fail here, including the ones whose answer is checked by a later line.
                    if jobs > 0
                        && let Some(said) = JOB_DID_NOT_RUN.iter().find(|s| answer.contains(**s))
                    {
                        failed.push(format!(
                            "`{line}` runs {jobs} job(s) and one did not run: its answer carries \
                             {said:?} ({:?})",
                            answer.trim()
                        ));
                    }
                }
                None => failed.push(format!("`{line}` produced no answer at all")),
            }
        }
    }

    // **Nothing may have died** (milestone 233), which is the ratchet milestone 230's lane
    // identified and deliberately left, because it would have been red on both architectures until
    // `login` was fixed. It was: `login` faulted at `_start` on every interactive boot, on both
    // ISAs, for an unknown length of time, while every check above passed and the progenitor went on printing
    // a line about a login service.
    //
    // **The whole transcript, not the boot**, because the typed script is where a death would be
    // most surprising. Nothing in `SWISH_CHECK_SCRIPT` traps on purpose: the three lines that fail
    // (`wc` and `doc` with nothing named, `least_authority_demo` with no argument) are all refusals, two at the
    // prompt before anything is spawned and one an ordinary non-zero exit, and `rm gate.txt`'s
    // refusal is an answer rather than a fault. `echo $?` reading `2` right after `least_authority_demo` is this
    // gate's own proof of that distinction: a thread the kernel killed does not get to set a status.
    // A trap in any of them would be a real regression rather than a false positive here.
    //
    // **What a deliberate trap does was measured rather than assumed** (milestone 233), because
    // milestone 230's lane named it as the thing it could not cheaply find out. `least_authority_demo` was
    // patched to `supervision_protocol::fail()` on `least_authority_demo 5` and this gate run against it. Two
    // results, and the second is the more interesting one:
    //
    //   1. This check fires, naming the thread and the reason, so it is a check that can fail
    //      rather than one that only ever passes. That mattered: it was written against a tree
    //      where `login` had just stopped dying, so nothing else would have exercised it.
    //   2. **The prompt never comes back.** The run also failed with "the prompt never came back
    //      to take `least_authority_demo 6`", because the shell waits on the result endpoint of a job that
    //      faulted instead of sending, and nothing wakes that wait. A spawned command that traps
    //      hangs the shell rather than returning a status. That is a real limitation this gate now
    //      makes visible, and it is `components/src/swish.rs`'s to carry rather than this file's.
    //
    // The first two of `KERNEL_FAULT_TOKENS` rather than one string, and that constant's own doc
    // carries why. The same pair is what `kernel_wrote_during_boot` reads, which is the other half
    // of this: a fault during the boot is both a failure here and the one thing that can shuffle a
    // console line, so the two checks are looking at one fact from two sides.
    if let Some(at) = transcript.find(KERNEL_FAULT_TOKENS[0])
        && transcript[at..].contains(KERNEL_FAULT_TOKENS[1])
    {
        let line = transcript[at..].lines().next().unwrap_or("").trim_end();
        failed.push(format!(
            "the kernel reported killing a user thread during this run: {line:?}. Every \
             program this boot starts is supposed to survive it, and one that does not is \
             invisible everywhere else: the progenitor's own report says what the progenitor measured, not what \
             stayed alive. The transcript below has the fault's registers, and `llvm-objdump -d` \
             on the program at that `pc` names the function."
        ));
    }

    // **And the capability-slot gauge** (milestone 231), which is the other half of this pair of
    // milestones. Two claims, and neither is a margin picked out of the air.
    //
    // The line must be *there*, because a gauge that stopped printing is a gauge nobody would miss
    // until the wall arrived again, which is exactly how `CAPABILITY_TABLE_SLOTS` came to be raised
    // three times reactively. And it must not say `ABOVE`, which is the kernel's own word for a
    // boot that went past the peak `kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED` records. That
    // constant is a measurement rather than a target, so what this fails on is a recorded fact
    // going stale, not a boot getting close to something. The fix when it fires is to measure,
    // update the constant, and re-read the headroom arithmetic beside `CAPABILITY_TABLE_SLOTS`.
    match transcript.lines().rfind(|l| l.contains(SLOT_GAUGE)) {
        Some(line) => {
            eprintln!("swish-check ({arch}):{}", line.trim_end());
            // **On x86_64 this line is stale, and says so** (milestone 182). The gauge is printed
            // from the scheduler's idle loop, and x86_64's input driver polls COM1 and yields
            // rather than blocking (milestone 299), so once it starts the run queue is never empty
            // and the idle loop never runs again. What prints is the mark at the hand-over, before
            // the progenitor has built anything. A temporary instrument on 2026-09-19 read 17 of
            // 24 at this leg's peak; milestone 182's BUGS has it. The `ABOVE` check below cannot
            // fire here for the same reason, which is a gate that cannot fail, stated rather than
            // hidden.
            if x86 {
                eprintln!(
                    "swish-check (x86_64): that gauge is the mark at the hand-over, not the peak: \
                     the idle loop that prints it never runs again while the input driver polls"
                );
            }
            if line.contains("ABOVE") {
                failed.push(format!(
                    "this boot used more capability slots than the tree records: {:?}. The \
                     number beside CAPABILITY_TABLE_SLOTS in kernel/src/cap.rs is now stale; \
                     measure, update CAPABILITY_TABLE_PEAK_MEASURED, and re-read that constant's \
                     headroom arithmetic rather than raising the ceiling reflexively.",
                    line.trim()
                ));
            }
        }
        None => failed.push(format!(
            "the boot never printed {SLOT_GAUGE:?}. The kernel says this from the scheduler's \
             idle loop once the mark has settled (kernel::cap::report_peak), so either the boot \
             never idled or the gauge stopped being printed; the second is the one that matters, \
             because it is the only thing standing between this tree and a fourth reactive raise \
             of CAPABILITY_TABLE_SLOTS."
        )),
    }

    // SIGTERM rather than `kill()`'s SIGKILL on x86_64: that runner is `qemu-bounded.sh`, which
    // forwards TERM to QEMU (the other two runners `exec` the emulator, so the kill is QEMU's).
    if x86 {
        let _ = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status();
    } else {
        let _ = child.kill();
    }
    let _ = child.wait();
    let _ = reader.join();

    if failed.is_empty() && !fresh {
        eprintln!(
            "swish-check ({arch}): rebooted against the same disk, ran the two packages installed \
             before the reboot, removed one and saw its vouch gone while the other still ran, \
             rolled back, and ran it again"
        );
        return true;
    }
    if failed.is_empty() {
        // Summed from the lines this leg typed, so an omitted line (see [`swish_check_omits`]) and
        // a skipped `std_exerciser` take their jobs with them; every tagged line was checked above
        // for a job that did not run. Until 2026-09-26 this was a hand-kept word per leg (27 on
        // aarch64 with `std_exerciser`), which undercounted the real 45 by eighteen.
        let jobs: u32 = script
            .iter()
            .filter(|l| !skipped(l.typed))
            .map(|l| u32::from(l.jobs))
            .sum();
        if x86 {
            let omitted: Vec<&str> = script
                .iter()
                .map(|l| l.typed)
                .filter(|line| swish_check_omits(arch, line).is_some())
                .collect();
            eprintln!(
                "swish-check (x86_64): booted under OVMF from \\EFI\\BOOT\\BOOTX64.EFI; ran {} \
                 of {} lines, omitting {}: {:?}",
                script.len() - omitted.len(),
                script.len(),
                omitted.len(),
                omitted,
            );
        }
        let std_ran = if std_built {
            ", one of them a `std` program built in the layout nife's `std` reads"
        } else {
            ""
        };
        let network = if x86 {
            "refused the network to a program that did not declare it, ran bytes nobody vouched \
             for holding nothing the line did not grant but the two pages, vouched for them as \
             the owner and rolled the vouch back, "
        } else {
            "reached the network twice through the stack the progenitor built and refused it to a \
             program that did not declare it, ran bytes nobody vouched for holding nothing the line \
             did not grant but the two pages, vouched for them as the owner and rolled the vouch \
             back, "
        };
        eprintln!(
            "swish-check ({arch}): the prompt booted, piped, redirected, appended, named a \
             file to a reader, read the clock, timed a command with a clock of its own, kept \
             a declared second stream off the redirection, previewed a directory grant and \
             then removed exactly the name it designated through a caretaker the progenitor built for \
             that one command, swept a \
             match too large to hand over in batches whose authority is exactly what each was \
             designated, named a file whose name has a space in it, searched an installed \
             documentation store and got back pages a following line could then designate, \
             rendered one of those pages straight at the prompt with no `| wc` in front of it, ran \
             a && past a command that succeeded and not past one it refused, {network}stopped a \
             supervised job with ^C and tore down one that ignored it, and ran \
             {jobs} jobs through the progenitor's bounded job pool after the progenitor gave its \
             construction budget away{std_ran}"
        );
        return true;
    }
    eprintln!();
    eprintln!("--- swish-check ({arch}) transcript ---");
    eprintln!("{whole}");
    eprintln!("--- swish-check ({arch}) FAILED ---");
    for f in &failed {
        eprintln!("  {f}");
    }
    false
}

/// **The graphical leg** (milestone 177, option A): the same `--features shell` boot, with a
/// virtio-gpu attached instead of the plain UART pair, verified by reading the *screen* back
/// rather than a serial transcript.
///
/// # Two keystroke sources, one leg (milestone 192, option A)
///
/// [`Keystrokes::Device`] attaches a virtio-keyboard and presses a key with the QEMU monitor's
/// `sendkey`, which is milestone 177's shape. [`Keystrokes::Serial`] attaches **no** keyboard and
/// types the same byte down the guest's UART, which is the configuration every one of the three
/// target machines actually has: argon, radon and xenon all have a serial line and none has a
/// virtio-input device.
///
/// **The same two assertions cover both**, and that they can is the claim. What reaches the screen
/// is `line_editor`'s echo of one `OP_BYTES` `CALL` on one endpoint, and neither this leg nor
/// anything past `kbd_ep` in the guest can tell which program made that `CALL`. If a future change
/// made the graphical stack depend on the keystroke's source, exactly one of these two runs would
/// go red.
///
/// # Why this cannot be [`swish_check_leg`] with two env vars added
///
/// [`swish_check_leg`]'s whole verification is a transcript piped over the UART: `console`/`input`
/// are exactly the two programs the graphical boot does not spawn (design/roadmap/
/// 177-graphical-interactive-boot.md's own finding), so there is no serial channel left to pipe.
/// The only observable surface is what a person looking at the screen would see, which on this
/// machine means a `screendump` over the QEMU monitor (`NIFE_GPU_MON`) and a real key press
/// (`sendkey`) for the same reason `kernel/src/user/display_tests.rs`'s own keyboard test needs the
/// host to press one: nothing in the guest can.
///
/// # Why this proves less than [`swish_check_leg`], and on purpose
///
/// [`SWISH_CHECK_SCRIPT`] is many lines because it is the whole redirection/pipeline/glob/manual
/// story, and every one of those checks a known **string**. There is no equivalent "the known
/// picture" to check against here: the boot banner's exact wrapped, scrolled position in an 18x8
/// grid is a function of wording nobody wants two copies of (one in `crates/system_initializer`,
/// one in this gate), and predicting it exactly is real work for no claim this milestone needs to
/// make. What this leg proves is the thing milestone 177 actually adds: the graphical stack wires
/// up with no capability-slot collision (a collision fails the boot in total silence, so *any*
/// prompt reaching the screen disproves one) and a real keystroke, through `keyboard_driver`'s new direct
/// `CALL` to `line_editor` and back out through `display_terminal`, reaches the screen. Proving the
/// rest of [`SWISH_CHECK_SCRIPT`] against a graphical prompt is real, scoped-out follow-on work,
/// not a gap this leg pretends is closed.
///
/// It looks for `$ ` (the exact two bytes `swish` prints for every prompt, `proto`-unrelated to
/// anything this leg computed in advance) anywhere in the decoded grid, not at a predicted row: a
/// terminal this small scrolls before the banner finishes, and which row the prompt lands on is
/// exactly the thing not worth predicting twice. Finding it at all is the proof that the progenitor built the
/// console... no: that it built `line_editor`, `display_terminal` and the display driver, wired
/// them to each other with no wrong slot, and that `swish` is alive and printing through them.
/// Finding `$ a` after `sendkey "a"` is the proof that a keystroke makes the same round trip back:
/// `keyboard_driver` (`MODE_DIRECT`) into `line_editor`, echoed out through `display_terminal`.
/// Which keystroke source [`swish_check_leg_graphical`] wires up. See its doc; the fork is
/// design/roadmap/192-keyboard-on-real-silicon.md's, and the kernel's own copy of it is
/// `kernel::user::KeystrokeSource`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Keystrokes {
    /// A virtio-input device, pressed with the monitor's `sendkey`. Milestone 177.
    Device,
    /// The guest's own UART, typed down the child's stdin (`-serial stdio`). Milestone 192,
    /// option A, and the only source any of the three real machines has.
    Serial,
}

fn swish_check_leg_graphical(riscv: bool, keystrokes: Keystrokes) -> bool {
    use std::io::Write;
    use std::time::{Duration, Instant};

    let arch = if riscv { "riscv64" } else { "aarch64" };
    let source = match keystrokes {
        Keystrokes::Device => "a keyboard",
        Keystrokes::Serial => "no keyboard, keystrokes over the UART",
    };
    eprintln!();
    eprintln!(
        "--- swish-check ({arch}, graphical): boot `--features shell` with a GPU and {source} ---"
    );

    let target = if riscv { RISCV_TARGET } else { TARGET };
    let built = if riscv {
        redoxfs_server_build(RISCV_TARGET) && mkdisk() && mkredoxfs() && initrd_riscv()
    } else {
        redoxfs_server_build(TARGET) && mkredoxfs() && mkdisk() && user()
    } && run(
        "cargo",
        &[
            "build",
            "-p",
            "kernel",
            "--features",
            "shell",
            "--target",
            target,
        ],
    );
    if !built {
        return false;
    }

    let sock = gpu_mon_socket(&format!("{arch}-swish-check"));
    let _ = std::fs::remove_file(&sock);

    let mut cmd = Command::new(if riscv {
        "helpers/qemu-runner-riscv64.sh"
    } else {
        RUNNER
    });
    cmd.arg(format!("target/{target}/{}/kernel", profile_dir()));
    cmd.env(
        "NIFE_INITRD",
        if riscv {
            riscv_initrd_path()
        } else {
            initrd_path()
        },
    );
    cmd.env("NIFE_DISK", disk_path());
    // **The serial arm attaches no virtio-rng**, and that is the point of it rather than an
    // omission: `NIFE_RNG` is a QEMU-only stopgap (DECISIONS §120) and none of the three target
    // machines has such a device, so an option-A leg standing in for a board should not have one
    // either. The device arm keeps it, unchanged, because that is milestone 177's leg. (A trap in
    // the progenitor whenever a virtio-rng was attached, recorded here on 2026-09-02, no longer
    // reproduces: on 2026-09-19 both arms reached a prompt, the device arm with the RNG attached.)
    if keystrokes == Keystrokes::Device {
        cmd.env("NIFE_RNG", "1");
    }
    // The flags [`swish_check_leg`] never sets: a virtio-gpu and (in the device arm) a
    // virtio-keyboard, the same devices `cargo xtask test` already attaches, read by
    // `helpers/qemu-runner-*.sh` exactly the way they always have been (milestone 177 (wire the graphical terminal stack into the real interactive boot) changed what
    // *the progenitor* does with them existing, not how they get attached).
    cmd.env("NIFE_GPU", "1");
    if keystrokes == Keystrokes::Device {
        cmd.env("NIFE_KEYBOARD", "1");
    }
    cmd.env("NIFE_GPU_MON", &sock);
    // stdout is never read: the answer this leg checks is on the screen, not on the wire. Kernel
    // boot messages before userspace exists still reach the host's own terminal, which is useful
    // to a person reading a failure and touches nothing this leg checks.
    //
    // stdin is the keyboard in [`Keystrokes::Serial`] (the runner passes `-serial stdio`, so a
    // byte written here arrives in the guest's UART receive FIFO and raises its interrupt) and is
    // null otherwise, which is milestone 177's shape unchanged.
    cmd.stdin(if keystrokes == Keystrokes::Serial {
        std::process::Stdio::piped()
    } else {
        std::process::Stdio::null()
    });
    cmd.stdout(std::process::Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("swish-check (graphical): failed to start the runner: {e}");
            return false;
        }
    };

    // `a`..`z`, `0`..`9`, space and `$`: every byte this leg's own checks look for, plus enough of
    // the alphabet that a decode failure names the wrong character instead of silently reading `?`
    // for one this leg simply never bothered to include.
    let mut alphabet: Vec<u8> = (b'a'..=b'z').collect();
    alphabet.extend(b'0'..=b'9');
    alphabet.push(b' ');
    alphabet.push(b'$');

    let shot = workspace_root().join(format!("target/gpu-swish-check-{arch}.ppm"));
    let deadline = Instant::now() + Duration::from_secs(SWISH_CHECK_BOOT_SECS);
    let mut prompt_row: Option<String> = None;
    while Instant::now() < deadline && prompt_row.is_none() {
        if screendump(&sock, &shot)
            && let Ok(bytes) = std::fs::read(&shot)
            && let Ok(rows) = scanout_rows(&bytes, &alphabet)
        {
            prompt_row = rows.into_iter().find(|r| r.contains("$ "));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let Some(before) = prompt_row else {
        let _ = child.kill();
        let _ = child.wait();
        eprintln!(
            "swish-check ({arch}, graphical): no `$ ` prompt appeared on the scanout within \
             {SWISH_CHECK_BOOT_SECS}s (see {})",
            shot.display(),
        );
        return false;
    };
    eprintln!("swish-check ({arch}, graphical): prompt found: {before:?}");

    // The one keystroke this leg types, the same key (and the same reason) the kernel test's own
    // keyboard test uses: `video_terminal::script::HOST_KEY` is the one definition of which key,
    // so a driver that mapped the evdev code wrong fails in exactly one place instead of two. The
    // serial arm sends the same key as the byte it already is, since a UART carries no scancode
    // for a keymap to get wrong; `HOST_KEY_BYTE` is that byte, defined beside `HOST_KEY` so the
    // two spellings of one key cannot drift.
    match keystrokes {
        Keystrokes::Device => sendkey(&sock, video_terminal::script::HOST_KEY),
        Keystrokes::Serial => {
            let Some(stdin) = child.stdin.as_mut() else {
                let _ = child.kill();
                let _ = child.wait();
                eprintln!("swish-check ({arch}, graphical): the runner has no stdin to type into");
                return false;
            };
            if let Err(e) = stdin
                .write_all(&[video_terminal::script::HOST_KEY_BYTE])
                .and_then(|()| stdin.flush())
            {
                let _ = child.kill();
                let _ = child.wait();
                eprintln!("swish-check ({arch}, graphical): could not type into the UART: {e}");
                return false;
            }
        }
    }

    let want = format!("$ {}", video_terminal::script::HOST_KEY);
    let deadline = Instant::now() + Duration::from_secs(SWISH_CHECK_LINE_SECS);
    let mut typed_row: Option<String> = None;
    while Instant::now() < deadline && typed_row.is_none() {
        if screendump(&sock, &shot)
            && let Ok(bytes) = std::fs::read(&shot)
            && let Ok(rows) = scanout_rows(&bytes, &alphabet)
        {
            typed_row = rows.into_iter().find(|r| r.contains(&want));
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&sock);

    match typed_row {
        Some(after) => {
            let by = match keystrokes {
                Keystrokes::Device => "`keyboard_driver`'s direct CALL to `line_editor`",
                Keystrokes::Serial => "`input`'s CALL to `line_editor`, over the UART",
            };
            eprintln!(
                "swish-check ({arch}, graphical): the prompt reached the screen through \
                 `display_terminal`, and a key press reached it back through {by}: {after:?}"
            );
            true
        }
        None => {
            let blame = match keystrokes {
                Keystrokes::Device => {
                    "`keyboard_driver` came up but its CALL to `line_editor` is not reaching it, \
                     or the host's `sendkey` is not reaching the device"
                }
                Keystrokes::Serial => {
                    "`input` came up but its CALL to `line_editor` is not reaching it, or the \
                     byte written to the runner's stdin is not reaching the guest's UART"
                }
            };
            eprintln!(
                "swish-check ({arch}, graphical): the prompt appeared ({before:?}) but the key \
                 press ({:?}) never echoed back within {SWISH_CHECK_LINE_SECS}s (see {}): {blame}",
                video_terminal::script::HOST_KEY,
                shot.display(),
            );
            false
        }
    }
}

/// **What the prompt printed in response to the first `line` at or after `from`**, plus where to
/// resume looking. `None` when that line is not in the transcript at all.
///
/// `kernel::user::pipeline_service::answer` does this inside the guest and this does it on the
/// host, for the same reason: an assertion should be able to name the command it is about instead
/// of counting lines.
fn swish_check_answer<'a>(
    transcript: &'a str,
    from: usize,
    line: &str,
) -> Option<(&'a str, usize)> {
    let echo = format!("$ {line}\n");
    let at = from + transcript[from..].find(&echo)? + echo.len();
    let rest = &transcript[at..];
    // The answer runs to the next prompt, which is always at the start of a line, and `rest` begins
    // at one because the echo consumed its own newline. So a command that printed nothing has an
    // empty answer rather than swallowing the line after it, which is the case `>` and `>>` are
    // and which the first version of this got wrong.
    let end = if rest.starts_with("$ ") {
        0
    } else {
        rest.find("\n$ ").map(|e| e + 1).unwrap_or(rest.len())
    };
    Some((&rest[..end], at + end))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The transcript milestone 230's first CI step produced**, copied out of run 33702132439
    /// rather than reconstructed, because the exact shape of the shuffle is the whole point.
    ///
    /// Two writers on one UART with nothing arbitrating: the kernel's user-fault printer and the
    /// userspace `console` server. `progenitor: construction budget dropped...` is spliced through the
    /// kernel's register line one and two characters at a time. Every byte of both is present and
    /// in order.
    const INTERLEAVED_CI_TRANSCRIPT: &str = "\
  user thread 17 killed: scause 0x3 (code 3)
    pc 0x0000000000406aa0   stval 0x000000000i0406aa0   usern sp 0x0000000000500da0
it:   cthe kernel is fine.
onstruction budget dropped; retype answers NoSuchSlot
progenitor: every program measured against the archive table

nife capability shell. naming a resource in a command IS granting it.
";

    /// **The shuffle that defeated the second attempt**, from run 33707574930, the same code that
    /// had passed run 33705237435 half an hour earlier. Evidence that the severity is
    /// nondeterministic: `construction budget dropped` survives here only as a longest run of
    /// `const`, and the kernel's own `the kernel is fine.` is destroyed in the same breath, which is
    /// what left a tolerance keyed on the kernel's signature with nothing to key on. Dropping that
    /// signature is what lets this one read again; the signature was never the safety, the ordering
    /// in [`boot_claim`] is.
    const SHREDDED_CI_TRANSCRIPT: &str = "\
  user thread 17 killed: scause 0x3 (code 3)
    pc 0x0000000000406aa0   stval 0x0000000000406aa0   user sp 0x0000000000500da0
  the kernel iis fnit: constiner.
uction budget dropped; retype answers NoSuchSlot
progenitor: every program measured against the archive table

nife capability shell. naming a resource in a command IS granting it.
";

    /// A clean boot reads exactly; a mildly shuffled one still reads, and says what it stepped over.
    #[test]
    fn a_claim_survives_being_interleaved_with_another_writer() {
        let clean = "progenitor: construction budget dropped; retype answers NoSuchSlot\n";
        assert!(matches!(
            boot_claim(
                clean,
                "construction budget dropped; retype answers NoSuchSlot",
                "construction budget NOT dropped"
            ),
            BootClaim::Affirmed(None)
        ));
        assert!(matches!(
            boot_claim(
                INTERLEAVED_CI_TRANSCRIPT,
                "construction budget dropped; retype answers NoSuchSlot",
                "construction budget NOT dropped"
            ),
            BootClaim::Affirmed(Some(_))
        ));
    }

    /// The run that failed CI reads again, and the fix is a deletion rather than an addition.
    #[test]
    fn the_shuffle_that_failed_ci_reads_again() {
        assert!(matches!(
            boot_claim(
                SHREDDED_CI_TRANSCRIPT,
                "construction budget dropped; retype answers NoSuchSlot",
                "construction budget NOT dropped"
            ),
            BootClaim::Affirmed(Some(_))
        ));
    }

    /// **The one that matters: a shuffle too severe to read is not a failure.**
    ///
    /// Two attempts at hardening a matcher were each defeated by a shuffle worse than the fixture
    /// they were tested against, so this stops betting on the matcher. What passes this transcript
    /// is not a cleverer match: it is that neither sentence is readable AND the kernel was
    /// demonstrably writing during the boot, which is the only condition under which a line can go
    /// missing without the progenitor having gone quiet.
    ///
    /// Synthetic, and said so: no CI run has produced a shuffle this bad, and the point is that
    /// nothing rules one out. Every character of the marker is present and in order, spread through
    /// far more foreign text than one fault report accounts for.
    #[test]
    fn a_shuffle_too_severe_to_read_is_reported_rather_than_failed() {
        let mut shredded = String::from("  user thread 17 killed: scause 0x3 (code 3)\n");
        for c in "construction budget dropped; retype answers NoSuchSlot".chars() {
            shredded.push(c);
            shredded.push_str("    pc 0x0000000000406aa0   stval 0x0000000000406aa0\n");
        }
        shredded
            .push_str("\nnife capability shell. naming a resource in a command IS granting it.\n");
        let BootClaim::Unreadable { longest_run } = boot_claim(
            &shredded,
            "construction budget dropped; retype answers NoSuchSlot",
            "construction budget NOT dropped",
        ) else {
            panic!("a transcript this badly shuffled cannot be read and must not be failed");
        };
        // "co", from `code 3` in the fault line rather than from the progenitor: with the marker spread this
        // thin, the longest surviving run of it is noise. Which is the fact worth reporting.
        assert!(
            longest_run.len() <= 3,
            "the longest surviving run is reported so a reader can judge the damage, got \
             {longest_run:?}"
        );
    }

    /// **The teeth.** The progenitor printing the failing answer is the thing this check exists to catch, and
    /// it survives every amount of shuffling elsewhere in the transcript, because the search for it
    /// is exact and interleaving can destroy a string but never create one.
    #[test]
    fn init_reporting_the_failing_answer_is_a_failure_however_shuffled_the_rest_is() {
        let denied = format!(
            "{}progenitor: construction budget NOT dropped; it can still build\n",
            SHREDDED_CI_TRANSCRIPT
        );
        assert!(matches!(
            boot_claim(
                &denied,
                "construction budget dropped; retype answers NoSuchSlot",
                "construction budget NOT dropped"
            ),
            BootClaim::Denied
        ));
    }

    /// **And the vacuity guard.** A boot where the progenitor simply stopped reporting, with nothing else
    /// writing the UART, has nothing that could have shuffled the line away, so its absence is real
    /// and this fails. Without this, "cannot read it" would be a way to pass against a check that
    /// had quietly stopped checking anything.
    #[test]
    fn a_silent_init_fails_when_nothing_else_was_writing() {
        let quiet = "\
  uart irq: source 10 (machine description)
progenitor: every program measured against the archive table

nife capability shell. naming a resource in a command IS granting it.
";
        assert!(matches!(
            boot_claim(
                quiet,
                "construction budget dropped; retype answers NoSuchSlot",
                "construction budget NOT dropped"
            ),
            BootClaim::Silent
        ));
    }

    /// A fault **after** the prompt is out cannot explain a boot line that was read before it, so it
    /// does not license [`BootClaim::Unreadable`]. Otherwise any typed command that traps on purpose
    /// would switch the boot checks off for the rest of the run.
    #[test]
    fn a_fault_after_the_prompt_does_not_excuse_a_missing_boot_line() {
        let late = "\
progenitor: every program measured against the archive table

nife capability shell. naming a resource in a command IS granting it.
$ outlaw
  user thread 22 killed: scause 0x3 (code 3)
  the kernel is fine.
";
        assert!(matches!(
            boot_claim(
                late,
                "construction budget dropped; retype answers NoSuchSlot",
                "construction budget NOT dropped"
            ),
            BootClaim::Silent
        ));
    }

    /// **Every program the shell can spawn is spawned by `script/swish-check`, or says why not**
    /// (milestone 150). Before this, a program's presence in the booted system was proven only by a
    /// transcript line somebody remembered to type, and three of thirteen had none. The check is a
    /// token match (the program's name as a whole word anywhere in a line), which is weaker than
    /// "the line ran it" and is enough to make forgetting loud.
    /// **The `x86_64` install line names the file the seed writes** (milestone 198 rung 3a): the two
    /// are spelled in two files, and a drift would make the line install nothing.
    #[test]
    fn the_greeting_install_line_names_the_seeded_file() {
        let line = format!("package install {}", crate::disk::DOWNLOADED_GREETING);
        assert!(SWISH_CHECK_SCRIPT.iter().any(|l| l.typed == line));
        assert!(swish_check_omits("aarch64", &line).is_some());
        assert!(swish_check_omits("x86_64", &line).is_none());
    }

    /// **A job count names a program** ([`Line`]'s doc): no line may claim more jobs than it has
    /// stages whose head is something the progenitor can build, a `grant_plan::Prog` or an
    /// installed package's path, after the `time` and `xargs` prefixes. A `caps` head is a preview
    /// and builds nothing. A bound from above only; a tag that is too low is the case no host test
    /// can see, and the transcript cannot either.
    #[test]
    fn a_job_count_names_a_program() {
        for l in SWISH_CHECK_SCRIPT.iter().chain(SWISH_CHECK_AFTER_REBOOT) {
            let heads = l
                .typed
                .split(['|', '&', ';'])
                .filter_map(|stage| {
                    stage
                        .split_whitespace()
                        .find(|w| *w != "time" && *w != "xargs")
                })
                // A token with a `/` in it runs a file's bytes (DECISIONS §219 D), which is the
                // shell's own test (`components/src/swish.rs`, `run`).
                .filter(|head| {
                    head.contains('/') || grant_plan::Prog::ALL.iter().any(|p| p.name() == *head)
                })
                .count();
            assert!(
                usize::from(l.jobs) <= heads,
                "`{}` claims {} job(s) and has {heads} stage(s) headed by a program",
                l.typed,
                l.jobs
            );
        }
    }

    #[test]
    fn every_spawnable_program_has_a_swish_check_line() {
        // Programs a transcript cannot drive, each with the reason. Empty since 2026-09-25, when
        // `SWISH_CHECK_INTERRUPTED` taught this gate to press `^C` and the two supervised
        // demonstrators left this list.
        const UNSCRIPTED: [&str; 0] = [];
        for p in grant_plan::Prog::ALL {
            let name = p.name();
            let scripted = SWISH_CHECK_SCRIPT
                .iter()
                .any(|l| l.typed.split_whitespace().any(|w| w == name));
            assert!(
                scripted != UNSCRIPTED.contains(&name),
                "`{name}`: {}",
                if scripted {
                    "scripted now, so take it off UNSCRIPTED"
                } else {
                    "the shell can spawn it and SWISH_CHECK_SCRIPT never does; add a line (see \
                     notes/adding-a-program.md), or add it to UNSCRIPTED with the reason"
                }
            );
        }
    }
}
