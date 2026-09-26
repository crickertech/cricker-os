//! **The timetable: scheduled execution where every entry is a grant** (milestone 129).
//!
//! A cron, and the inversion of one. Unix cron reads a text file and runs arbitrary commands as
//! ambient authority made periodic: whatever the crontab says happens, with the account's whole
//! reach behind it, and the crontab is therefore the attack surface. This reads a text file too and
//! the file looks similar, but an entry's command is a **grant expression** checked at registration
//! by `grant_plan::plan`, the same function the shell checks a prompt line with. What a scheduled
//! child holds is planned, printed, and fixed before the first tick.
//!
//! **The claim, in one sentence:** compromising this process yields the entries' summed endowments,
//! not the system, because there is no ambient authority for a scheduled child to fall back on and
//! this process holds nothing it was not handed.
//!
//! # What it holds, and that is the whole list
//!
//! - slot 0: the output endpoint (WRITE). Where the plan and the summary go, as `byte_sink_protocol` bytes.
//! - slot 1: an untyped budget (WRITE). What every instance is made of, what pays for the loader's
//!   own scratch mappings, and what a `--mem` entry's grant is carved from (nested inside that
//!   instance's own region rather than split from this budget directly; see `fire` and `BUGS`).
//! - slot 2: the child report endpoint (WRITE|GRANT), handed to each instance as its slot 0.
//! - slot 3: the supervision endpoint (READ|GRANT), placed in each instance's reserved fault slot
//!   so every scheduled child is born supervised (DECISIONS §26), and invoked with
//!   `Rendezvous::REAP` to collect the corpses (§32).
//! - `a0`: how many fires to perform before summarising and exiting. `0` means forever.
//! - `a2`: where a registration page is mapped, or `0` for none (milestone 129 (scheduled execution), §222 (who holds a
//!   user's schedule)). With none, the document is the compiled-in `timetable.conf`. With one, the
//!   timetable starts empty and its document is whatever a registrar last sent with `REPLACE`; see
//!   `timetable::registration` and "Replacement" below.
//! - `a1`: the length of the archive the spawn site mapped read-only at
//!   [`user_mode_runtime::initrd::INITRD_VA`]. **Not the initrd**: it holds exactly the programs this
//!   document will ever build, because the plan is computable before the first tick and so the
//!   endowment can be narrowed to it. This process audits that and says what it found, in the
//!   line after the plan.
//!
//! **It wants a bigger stack than a small program does**, and a spawn site has to say so: a
//! `grant_plan::Endowment` is about a kilobyte (mostly the name set a directory grant can carry) and
//! the plan holds one per entry, so the working set is tens of kilobytes rather than hundreds of
//! bytes. `system_tests/src/user/timetable_tests.rs` maps 32 pages and says why; eight died with a data
//! abort whose faulting address was the stack pointer, which is what a stack overflow looks like
//! from the kernel side and reads like a wild pointer if you have not seen it before.
//!
//! **And nothing else beyond that budget.** No clock page, no directory, no console, no network, no
//! device. That list is not modesty: it is why a scheduled `date` in `timetable.conf` is refused at
//! registration rather than run, and why the refusal names the timetable rather than the line.
//! `timetable::SHIPPED_HELD` is the one fact that has widened since milestone 129's first stratum:
//! this process now holds enough budget to back a `--mem` grant up to `SHIPPED_HELD.mem_pages`
//! pages for a single entry, and `timetable.conf`'s `at-boot memory_grant_depleter --mem 4` line is the proof.
//!
//! # The archive is narrowed to the plan, and this process says so
//!
//! `Registry::programs` is the complete set of programs the document will ever start, and it is
//! known before anything fires. So the spawn site builds an archive of exactly that set and hands
//! *that* over, rather than the initrd: after milestone 129's second stratum this process cannot
//! load `date`, `ps`, `swish` or anything else it will never run, even though it holds a working
//! image loader.
//!
//! It cannot narrow its own endowment, so what it does instead is **measure it** and print the
//! answer next to the plan (`timetable::Audit`). A scheduler handed the whole initrd still works
//! and says a different sentence, which is the property worth having: the width of the endowment is
//! a line on the console rather than a fact only the spawn site knows.
//!
//! # The loop, and the one thing it cannot do
//!
//! Poll the monotonic counter (ambient, `user_mode_runtime::monotonic_nanos`, no capability); fire what is
//! due; when the budget cannot back another instance, block on the supervision endpoint until a
//! corpse arrives and reclaim its region. That last step is the only blocking wait in the program,
//! which matters because **this kernel has exactly one wait point per process and no timed wait at
//! all** (milestone 106 is `NOT-STARTED` and gated on a decision). See `BUGS`.
//!
//! # Replacement
//!
//! A timetable spawned with a registration page is changed while it runs, by its session replacing
//! the whole document (§222). The loop checks the page's request word once per pass, which is one
//! load, and it has to be a poll: a blocking receive would stop it watching the clock (see
//! `timetable::registration`). A replacement is parsed, registered and resolved against the archive
//! in full before anything changes, so one that fails leaves the schedule in force running and
//! says why in the page. One that succeeds keeps the beat of every line it did not change, prints
//! its plan down [`OUT`] and into the page, and re-arms.
//!
//! Children already running when their entry is removed finish and are reaped as usual, because
//! the counts of outstanding children belong to the loop and not to the document.
//!
//! An empty replacement ends the process. Its session is kept alive by its live children (§16
//! (object revocation)), so a timetable left idling with nothing to fire would hold the session up
//! for no job at all. It answers, drains what is running, prints its summary, and exits.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist), with `crates/timetable` and
//! `components/timetable.conf` in one ruling, which is what a crate-and-program pair means. See the
//! crate's module docs for the argument and the refusals. Milestone 129's own block declined to
//! propose a name and said the eventual one was calef's.
//!
//! # BUGS
//!
//! - **It spins between fires, and that is a missing kernel primitive rather than a lazy loop.**
//!   There is no sleep, no timeout and no deadline anywhere in this kernel, so a process that wants
//!   to act at a time can only yield and re-read the counter. `Registry::next_deadline` already
//!   computes exactly what a timed wait would block until, so the fix is one line here once
//!   milestone 106's fork is decided; until then a running timetable costs a core's worth of yields.
//!   **This program is that fork's fifth consumer** (the block counts four: `net_stack`'s retransmit
//!   window, milestone 51's `thread::sleep`, `RECV`'s no-timeout limitation, and the shell's `^C`
//!   poll), and it is the first one whose *whole purpose* is to act at a time.
//!
//! - **Corpses are collected lazily, when their memory is needed.** Nothing reaps between fires,
//!   because reaping means blocking on the supervision endpoint and blocking means not watching the
//!   clock. So the failure counts this program reports lag reality until the budget runs down.
//!   A wait that returns on either a message or a deadline fixes this too, and it is the same fork.
//!
//! - **A hung scheduled child stops the whole timetable.** When the budget cannot back another
//!   instance the loop blocks on the supervision endpoint, and a livelocked child never sends a
//!   death message, so nothing arrives and nothing else fires. That is §32's watchdog case verbatim
//!   (`Rendezvous::REAP` collects corpses and refuses to kill, deliberately) and it is not this
//!   program's to fix: it waits for milestone 23. What it costs here is worth stating, because it is
//!   worse than it is for a shell: at a prompt the person who typed the command is sitting there and
//!   can press `^C`, and behind a schedule there is nobody.
//!
//! - **The archive holds every image the plan builds, and that is code, not authority.** A
//!   compromised timetable can load any program in its archive, not only the one an entry names.
//!   It gains nothing by it: a job holds exactly what `fire` or `fire_with_grant` endows, and
//!   nothing reads which image is running to decide that, so a second image adds only code to a
//!   process already running the attacker's. One image per entry was refused on 2026-09-26 for
//!   this reason (notes/scheduled-execution/one-image-per-entry.md).
//!
//! - **`--mem` entries are backed, and run one at a time, alone.** The roadmap's first sketch said
//!   to split the grant out of the instance's own region "so a single `DESTROY` still reclaims
//!   both", and that is wrong on its own terms: `regions::destroy_outcome` returns `Refused` for
//!   any region with a live child, Kani proves it, and `sched::reap_supervised` hands that refusal
//!   straight back, so a corpse whose region carries a nested grant can never be collected through
//!   `reap` until the grant is destroyed first, by its own separate capability.
//!
//!   The nesting survives the correction for a better reason: it is the only thing that can ever
//!   pair a death with a grant, because a builder is never told its child's tid
//!   (`supervision_protocol::build_child` hands back a TCB capability, and `abi::thread_control_block`
//!   has no method that reads one out), so the only fact this process has about a death is the tid
//!   the kernel stamped on it. `fire_with_grant` keeps the split untyped's own capability rather
//!   than `cap_delete`-ing it the way it does the region and the TCB, so `collect_grant` can destroy
//!   it later, by name.
//!
//!   **What decides *how many* `--mem` instances may be outstanding at once is that correlation,
//!   and the answer taken here is one.** A generation counter or a slot table could track more, but
//!   nothing here needs its child's tid for any other reason, so paying for one would be
//!   speculative machinery for a milestone whose document schedules exactly one such entry. With
//!   one, the pairing needs no bookkeeping at all: `_start` drains everything already outstanding,
//!   fires the grant-bearing instance alone, and blocks in `collect_grant` until it dies and its
//!   grant is reclaimed before returning to the loop. Nothing else in the document can be firing
//!   while that wait is blocked, which is what makes the next death on `DEATHS` unambiguous.
//!
//!   **The cost is real and is paid by every other entry, not by `--mem` ones.** While a grant is
//!   outstanding this process is blocked in one syscall and cannot poll the clock at all, so an
//!   interval entry due during that window is not skipped, it simply runs late once the loop
//!   resumes; several periods elapsing during a slow instance still produce one fire on resumption
//!   (`next_after`'s ordinary skip-not-catch-up rule, not a special case for this path).
//!   `timetable.conf`'s `at-boot memory_grant_depleter --mem 4` fires before the first `every 150ms` tick can
//!   even become due, so this cost is not exercised by the cross-ISA test; a document whose
//!   `--mem` entry shares the clock with a fast interval would pay it.
//!
//! - **Without a registration page, the document is compiled in.** `include_str!`, and the shipped
//!   boot-time test still runs that way. With a page the document is whatever the registrar sent,
//!   and persisting it is the registrar's job (§222's fifth sub-ruling: the session writes
//!   `crates/schedule_store`'s file, then replaces). No session registers into a timetable yet,
//!   because the durable session is for milestone 152 (durable delegation) to rebuild; the kernel test stands in.
//!
//! - **A registration is noticed by polling, not received.** The loop loads the page's request
//!   word once per pass. A `RECV` would block and stop the clock being watched, because a process
//!   has one wait point and there is no timed wait (milestone 106 (a wait that ends on either the interrupt or the deadline)). The cost is one load per pass
//!   on a loop that already spins; a deadline wait that also ends on a notification removes it.
//!
//! - **A replacement's printed plan is cut at the page.** The plan goes down [`OUT`] whole, and
//!   into the page up to `timetable::registration::BODY_MAX` bytes. Eight entries of long refusals
//!   could pass that; the verdict word still says what every entry became.
//!
//! - **The archive audit is printed only for the compiled-in document.** With a registrar, the
//!   archive is what the session lets its jobs run, not the plan of a document that has not
//!   arrived, so "exactly the programs its plan names" is not the right sentence to measure.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use grant_plan::spawnproto;
use timetable::{Registry, registration};
use user_mode_runtime::{cap_delete, exit, monotonic_nanos, reap, recv_fault, send, yield_now};

/// The document. Compiled in; see `BUGS`.
const CONFIG: &str = include_str!("../timetable.conf");

/// The output endpoint: the plan, and the summary. `byte_sink_protocol` bytes.
const OUT: u64 = 0;
/// The budget every instance is made of, and what pays this loader's scratch mappings.
const BUDGET: u64 = 1;
/// Handed to each instance as its slot 0, so a scheduled child can report its answer.
const CHILD_REPORT: u64 = 2;
/// Placed in each instance's reserved fault slot, and what corpses are collected through.
const DEATHS: u64 = 3;

/// Pages per instance region. Enough for a small program's segments, its stack, its address-space
/// tables and its TCB, and the same number `components/src/spawner.rs` arrived at for the same job.
///
/// A per-instance region rather than one shared pool is what makes a single `MemoryRegion::DESTROY`
/// reclaim a whole dead child, which is the property the reap depends on.
const INSTANCE_PAGES: u64 = 48;

/// How many times to retry a reap whose region still holds something that can run.
///
/// The same safety net as `components/src/job_undertaker.rs`'s, and the same reasoning: a `NotPermitted`
/// is a fact about the region's other residents rather than about this corpse, one preemption is
/// enough, and running out is still a loud failure because a corpse that never becomes collectable
/// is a leak that ends this process's memory a few fires later with nothing to point at.
const REAP_ATTEMPTS: usize = 1024;

/// Verdict codes on [`OUT`]'s stream, so a spawn site that reads nothing else can still tell what
/// happened. The plan and the summary are text; these are the two ways the program ends.
const E_CONFIG: u64 = 0xE300; // the document did not parse; the low byte is the line number
const E_ARCHIVE: u64 = 0xE301; // the initrd archive did not parse
const E_IMAGE: u64 = 0xE302; // a program an admitted entry names is not in the archive
const E_BUDGET: u64 = 0xE303; // the budget cannot back even one instance
const E_UNVOUCHED: u64 = 0xE304; // it was handed the run-unvouched capability; see `_start`

// The relation `grant_plan` cannot state without depending on `abi`, held here as every reader of
// the slot holds it (`components/src/swish.rs` does the same).
const _: () = assert!(spawnproto::RUN_UNVOUCHED_SLOT == abi::fault::FAULT_EP_SLOT - 1);
// No slot this program hands a job is the run-unvouched slot. The job's other capability is a
// region this program split for it, which cannot be the run-unvouched capability (see `_start`).
const _: () = assert!(CHILD_REPORT != spawnproto::RUN_UNVOUCHED_SLOT);

/// The images an admitted row will be built from, one slot per entry.
type Images = [Option<elf::Elf<'static>>; timetable::MAX_ENTRIES];

#[unsafe(no_mangle)]
pub extern "C" fn _start(fires_wanted: u64, initrd_len: u64, registration_page: u64) -> ! {
    // **A scheduled job never holds the run-unvouched capability**, which is how §220 (signed builds, and trusting a key is scoped) keeps its reach: dropping trust in a key has to reach every
    // program that could run an unvouched image, and a job firing on a schedule long after its
    // session's key was dropped is exactly the one it would miss. The capability is gate D2 of §219 (how the shell names an installed program to the spawner).
    //
    // Enforced here, once, rather than at each fire, and the argument is why once suffices. A job's
    // authority is built in `fire` and `fire_with_grant` from two sources only: [`CHILD_REPORT`],
    // and a region split for it from [`BUDGET`]. Neither can be the capability unless this process
    // holds it. It can only hold it if a spawn site put it there, because this program never
    // receives a capability after `_start` (it makes no `RECV_CAP`). So a timetable that does not
    // hold it at `_start` can never endow a job with it. The probe is sound only now, before
    // anything is allocated: a region split later could land in the slot and read as held.
    if user_mode_runtime::is_granted(spawnproto::RUN_UNVOUCHED_SLOT) {
        say(
            b"timetable: it holds the run-unvouched capability, which no scheduled job may hold, \
              so it runs nothing\n",
        );
        done(E_UNVOUCHED)
    }

    // SAFETY: forwarded from user_mode_runtime::initrd::initrd_bytes's own contract, the same one
    // `components/src/root_supervisor.rs` is started under. It named `components/src/builder.rs`
    // until milestone 295 retired that program; the contract is unchanged, only the sibling is.
    let archive = unsafe { user_mode_runtime::initrd::initrd_bytes(initrd_len) };

    // With a registrar, the timetable starts with nothing and waits to be told; without one, the
    // compiled-in document is the whole story, exactly as it was before §222.
    let text = if registration_page == 0 { CONFIG } else { "" };
    let doc = match timetable::parse(text) {
        Ok(d) => d,
        // The line number rides in the low byte, so a wrong document is findable from the verdict
        // alone even when nobody is reading the text stream.
        Err(e) => {
            say(b"timetable: the document does not parse: ");
            say(e.message().as_bytes());
            say(b"\n");
            done(E_CONFIG | (e.line() as u64 & 0xff));
        }
    };

    // **What this process holds, stated once, in the vocabulary registration checks against.**
    // Every field but `mem_pages` is false, and every one of them is a fact a reader can check
    // against the slot list in this module's header. `timetable::SHIPPED_HELD` is the one number
    // that has widened since milestone 129's first stratum, and this crate's own host test uses the
    // same constant so the two cannot drift apart. Widening it further is an edit here and a
    // visible change in the printed plan, which is the property worth having.
    let held = timetable::SHIPPED_HELD;

    let mut reg = Registry::register(&doc, held);

    // **The plan, before anything fires.** This is the milestone's claim in the form a person meets
    // it: what every scheduled child will hold, or why it will never run, printed while nothing has
    // happened yet. A crontab has nothing to print here.
    timetable::write_plan(&reg, &mut say);

    let Ok(fs) = nifefs::Fs::parse(archive) else {
        done(E_ARCHIVE)
    };

    // **What the archive it was handed reaches, next to what the plan will build.** The plan above
    // says what each scheduled child holds; this says what *this* process holds, and the two are
    // different questions. A scheduler handed the whole initrd has a one-program plan and a
    // capability that reaches every program in the tree, and nothing in the plan would say so.
    //
    // It measures rather than enforces, because a process cannot narrow its own endowment: the
    // width is the spawn site's decision (`system_tests/src/user/timetable_tests.rs` builds a sub-archive
    // from exactly `Registry::programs`), and saying it out loud is what makes the decision
    // checkable from in here rather than only from out there. Not with a registrar: see `BUGS`.
    if registration_page == 0 {
        let mut audit = timetable::Audit::of(&reg);
        for entry in fs.entries() {
            if let Some(name) = entry.name_str() {
                audit.saw(name);
            }
        }
        audit.write(&mut say);
    }

    // Resolve every admitted entry's program **now**, so a plan that names a program the archive
    // does not carry fails loudly at startup rather than as a fire that quietly does not happen.
    // This is also the moment `Registry::programs`' claim becomes checkable: nothing after this
    // point looks anything else up.
    let mut images: Images = match resolve(&reg, &fs) {
        Ok(images) => images,
        Err(i) => {
            if let Some(e) = reg.rows()[i].endowment() {
                say(b"timetable: no such program in the archive: ");
                say(e.prog.name().as_bytes());
                say(b"\n");
            }
            done(E_IMAGE)
        }
    };

    // The end of the plan, and the start of the running. One line, because the plan and everything
    // after it travel down one endpoint and a reader has to know where one stops: `kernel/src/user/
    // timetable_tests.rs` reads to exactly this line, which is also how a person reading a console
    // knows nothing had fired before it.
    say(b"timetable: armed\n");

    reg.arm(monotonic_nanos());

    // Which of the two document buffers the registry in force borrows from; see [`DOCUMENTS`].
    let mut current = 0usize;
    let mut answered = 0u64;

    let mut fired = 0u64;
    let mut outstanding = 0u64;
    let mut exits = 0u64;
    let mut faults = 0u64;

    while fires_wanted == 0 || fired < fires_wanted {
        if registration_page != 0
            && replace_if_asked(
                registration_page,
                &mut answered,
                &mut current,
                &mut reg,
                &mut images,
                &fs,
                held,
            )
        {
            // Emptied: nothing more fires, and what is running finishes below.
            break;
        }
        let now = monotonic_nanos();
        let mut any = false;
        while let Some(i) = reg.due(now) {
            any = true;
            // Several entries can come due in one pass, so the count has to be checked **inside**
            // the pass and not only around it. Without this a run asked for four fires can perform
            // five, and the fifth child blocks forever on a report nobody is left to take, which
            // shows up as a hang rather than as an off-by-one.
            if fires_wanted != 0 && fired >= fires_wanted {
                break;
            }
            let Some(elf) = images[i].as_ref() else {
                continue;
            };
            let Some(e) = reg.rows()[i].endowment() else {
                continue;
            };

            if e.mem_pages > 0 {
                // **Exclusive.** See `BUGS`: a `--mem` grant is nested inside its own instance's
                // region, and the only signal that ties a death to a grant is a refused reap, which
                // is unambiguous only when nothing else is outstanding to blame it on. So drain
                // whatever is already running, fire this one alone, and wait for it to die and its
                // grant to be reclaimed before anything else in this document fires again.
                while outstanding > 0 {
                    collect(&mut exits, &mut faults);
                    outstanding -= 1;
                }
                let Some(mem_slot) = fire_with_grant(elf, e.arg, e.mem_pages) else {
                    say(b"timetable: the budget cannot back one instance\n");
                    done(E_BUDGET)
                };
                fired += 1;
                collect_grant(&mut exits, &mut faults, mem_slot);
                continue;
            }

            // Fire. If the budget cannot back another instance, block until a corpse comes back and
            // its region with it, then try once more. A second failure is a budget too small for
            // even one instance, which is a wiring error rather than congestion.
            if !fire(elf, e.arg) {
                if outstanding == 0 {
                    // Nothing is out, so there is nothing to wait for: the budget is too small for
                    // even one instance, which is a wiring error rather than congestion.
                    say(b"timetable: the budget cannot back one instance\n");
                    done(E_BUDGET)
                }
                collect(&mut exits, &mut faults);
                outstanding -= 1;
                if !fire(elf, e.arg) {
                    say(b"timetable: the budget cannot back one instance\n");
                    done(E_BUDGET)
                }
            }
            outstanding += 1;
            fired += 1;
        }
        if !any {
            // Nothing is due. There is no timed wait in this kernel, so this is a yield and not a
            // sleep; see `BUGS`.
            yield_now();
        }
    }

    // Drain: every child this timetable started is collected before it reports, so the counts it
    // prints are complete rather than whatever had happened to arrive.
    while outstanding > 0 {
        collect(&mut exits, &mut faults);
        outstanding -= 1;
    }

    say(b"timetable: ");
    say_num(fired);
    say(b" fires, ");
    say_num(exits);
    say(b" clean exits, ");
    say_num(faults);
    say(b" faults\n");
    done(0)
}

/// **Resolve every admitted row's program in the archive**, or the index of the first that is not
/// there. Nothing is parsed lazily later: a plan that names a missing program is caught here.
fn resolve(reg: &Registry<'_>, fs: &nifefs::Fs<'static>) -> Result<Images, usize> {
    let mut images: Images = [const { None }; timetable::MAX_ENTRIES];
    for (i, row) in reg.rows().iter().enumerate() {
        let Some(e) = row.endowment() else { continue };
        let Some(bytes) = fs.read(e.prog.name()) else {
            return Err(i);
        };
        let Ok(elf) = elf::Elf::parse(bytes) else {
            return Err(i);
        };
        images[i] = Some(elf);
    }
    Ok(images)
}

/// **The two buffers a replacement's document is copied into**, alternately.
///
/// A [`Registry`] borrows its document's bytes, and the page cannot be what it borrows: the reply
/// overwrites the page with the plan, and a registrar may start staging the next document the
/// moment it has read the reply. So the document is copied out first. Two buffers because the
/// replacement must be registered and resolved in full while the registry in force still borrows
/// the other one, and only then may it replace it (§222's all-or-nothing sub-ruling).
static mut DOCUMENTS: [[u8; registration::BODY_MAX]; 2] = [[0; registration::BODY_MAX]; 2];

/// **Answer a replacement, if the registrar has asked for one since the last answer.**
///
/// Everything that can refuse happens before anything changes: the length, the text, the parse,
/// the registration and the archive lookup. Only then is the registry in force swapped, armed with
/// [`Registry::arm_after`] so unchanged lines keep their beat, and its plan printed down [`OUT`]
/// and into the page. The reply word is written last, with release ordering, so a registrar that
/// sees it sees everything before it.
///
/// Returns `true` when the replacement was an empty document: the caller stops firing, drains what
/// is running, and exits ([`registration::STATUS_EMPTIED`] says why an idle timetable must not stay).
fn replace_if_asked(
    page: u64,
    answered: &mut u64,
    current: &mut usize,
    reg: &mut Registry<'static>,
    images: &mut Images,
    fs: &nifefs::Fs<'static>,
    held: timetable::Held,
) -> bool {
    use core::sync::atomic::{AtomicU64, Ordering};
    // SAFETY: the spawn site mapped one writable page at `page` for exactly this protocol, and it
    // stays mapped for this process's life. Its first two header words are only ever accessed as
    // atomics, by both sides.
    let request = unsafe { &*((page + registration::REQUEST as u64) as *const AtomicU64) };
    // SAFETY: the same page and the same rule, for the reply word.
    let reply = unsafe { &*((page + registration::REPLY as u64) as *const AtomicU64) };
    // PAIR: the registrar's release store of the request word, after it staged the document.
    let word = request.load(Ordering::Acquire);
    let seq = registration::sequence(word);
    if seq == *answered {
        return false;
    }
    *answered = seq;
    // SAFETY: as above, the whole page is ours to read and write while the registrar waits for the
    // reply word, which is the protocol's one rule for the other side.
    let body =
        unsafe { core::slice::from_raw_parts_mut(page as *mut u8, registration::PAGE_BYTES) };
    // Every path answers in the page first and then speaks down `OUT`, never the other way round.
    let answer = |body: &mut [u8], status: u64, detail: u64, verdicts: u64, plan: usize| {
        body[registration::STATUS..registration::STATUS + 8].copy_from_slice(&status.to_le_bytes());
        body[registration::DETAIL..registration::DETAIL + 8].copy_from_slice(&detail.to_le_bytes());
        body[registration::VERDICTS..registration::VERDICTS + 8]
            .copy_from_slice(&verdicts.to_le_bytes());
        body[registration::PLAN_LEN..registration::PLAN_LEN + 8]
            .copy_from_slice(&(plan as u64).to_le_bytes());
        reply.store(seq, Ordering::Release);
    };

    if registration::operation(word) != registration::REPLACE {
        answer(body, registration::STATUS_UNKNOWN_OPERATION, 0, 0, 0);
        say(b"timetable: a registration asked for something other than a replacement\n");
        return false;
    }
    let len = u64::from_le_bytes(
        body[registration::LEN..registration::LEN + 8]
            .try_into()
            .unwrap(),
    );
    if len as usize > registration::BODY_MAX {
        answer(body, registration::STATUS_MALFORMED, 0, 0, 0);
        say(b"timetable: a replacement longer than the page, refused whole\n");
        return false;
    }
    let len = len as usize;
    let spare = 1 - *current;
    // SAFETY: `spare` is the buffer the registry in force does not borrow (see `DOCUMENTS`). The
    // registry that last borrowed it was replaced, and so dropped, before `current` moved off it.
    let text: &'static [u8] = unsafe {
        let buf = core::ptr::addr_of_mut!(DOCUMENTS[spare]).cast::<u8>();
        core::ptr::copy_nonoverlapping(body[registration::BODY..].as_ptr(), buf, len);
        core::slice::from_raw_parts(buf, len)
    };
    let Ok(text) = core::str::from_utf8(text) else {
        answer(body, registration::STATUS_MALFORMED, 0, 0, 0);
        say(b"timetable: a replacement that is not text, refused whole\n");
        return false;
    };
    let doc = match timetable::parse(text) {
        Ok(d) => d,
        Err(e) => {
            answer(body, registration::STATUS_PARSE, e.line() as u64, 0, 0);
            say(b"timetable: the replacement does not parse, and the schedule in force is unchanged: ");
            say(e.message().as_bytes());
            say(b"\n");
            return false;
        }
    };
    if doc.entries().is_empty() {
        answer(body, registration::STATUS_EMPTIED, 0, 0, 0);
        say(b"timetable: the document is empty, so this timetable exits once its running jobs finish\n");
        return true;
    }
    let mut next = Registry::register(&doc, held);
    let next_images = match resolve(&next, fs) {
        Ok(images) => images,
        Err(i) => {
            answer(body, registration::STATUS_NO_IMAGE, i as u64, 0, 0);
            say(
                b"timetable: the replacement names a program this timetable cannot load, and the \
                  schedule in force is unchanged\n",
            );
            return false;
        }
    };

    // Committed from here: nothing below can refuse.
    let kept = next.arm_after(reg, monotonic_nanos());
    *reg = next;
    *images = next_images;
    *current = spare;

    // The reply goes into the page before anything goes down `OUT`, so a registrar that waits on
    // the output line, as the kernel test does, finds the reply already there.
    let mut plan = 0usize;
    timetable::write_plan(reg, &mut |bytes: &[u8]| {
        let room = registration::BODY_MAX - plan;
        let n = bytes.len().min(room);
        body[registration::BODY + plan..registration::BODY + plan + n].copy_from_slice(&bytes[..n]);
        plan += n;
    });
    let verdicts = registration::verdicts(reg, kept);
    answer(body, registration::STATUS_REPLACED, 0, verdicts, plan);
    timetable::write_plan(reg, &mut say);
    say(b"timetable: armed\n");
    false
}

/// Build one instance in its own region and start it with `arg`.
///
/// It is endowed exactly two things and both are in the plan the timetable already printed: the
/// child report endpoint as its slot 0, and its supervision endpoint in the reserved fault slot,
/// which `START` reads and then clears so the child holds no authority on its own death channel.
///
/// Nothing is kept afterwards and there is nothing left worth keeping: the TCB capability is not the
/// thread, and since DECISIONS §32 the region capability is not the reap either. The pages come back
/// to this budget when the corpse is collected.
fn fire(elf: &elf::Elf, arg: u64) -> bool {
    let Ok(region) = supervision_protocol::memory_region_split(BUDGET, INSTANCE_PAGES) else {
        return false;
    };
    let Ok(child) = supervision_protocol::build_child(
        BUDGET,
        region,
        elf,
        &supervision_protocol::ChildEndowment {
            caps: &[(CHILD_REPORT, abi::rights::WRITE)],
            fault: Some(DEATHS),
            ..supervision_protocol::ChildEndowment::new(supervision_protocol::Retention::Nothing)
        },
    ) else {
        // The region is ours and the child does not exist, so hand the pages straight back rather
        // than leaking them into a budget that will refuse the next fire.
        supervision_protocol::memory_region_destroy(region);
        return false;
    };
    if !supervision_protocol::start_child(child, 0, arg, 0) {
        supervision_protocol::memory_region_destroy(region);
        return false;
    }
    cap_delete(region);
    true
}

/// **Build and start a `--mem`-backed instance**, the exclusive sibling of [`fire`].
///
/// The grant is carved out of the instance's own region rather than out of [`BUDGET`] directly
/// (`BUGS` says why: it is the only thing that ever pairs a death with a grant), so the region is
/// sized `INSTANCE_PAGES + mem_pages` and the grant is a second split off *that*, leaving
/// `INSTANCE_PAGES` for the instance's own address space, frames, stack and TCB exactly as before.
///
/// Returns the grant's own capability, still held, on success. **This is deliberately not deleted
/// the way [`fire`] deletes `region`**: it is the caller's only way to reclaim the grant later, and
/// the caller is [`collect_grant`], called next and only next by this program's one call site.
fn fire_with_grant(elf: &elf::Elf, arg: u64, mem_pages: u64) -> Option<u64> {
    let Ok(region) = supervision_protocol::memory_region_split(BUDGET, INSTANCE_PAGES + mem_pages)
    else {
        return None;
    };
    let Ok(mem_slot) = supervision_protocol::memory_region_split(region, mem_pages) else {
        supervision_protocol::memory_region_destroy(region);
        return None;
    };
    let Ok(child) = supervision_protocol::build_child(
        BUDGET,
        region,
        elf,
        &supervision_protocol::ChildEndowment {
            // Slot 0: the report endpoint, as every instance gets. Slot 1: the grant, narrowed to
            // WRITE so the child may spend it and not lend it (the same narrowing
            // `system_initializer` gives a shell's `--mem` delegation).
            caps: &[
                (CHILD_REPORT, abi::rights::WRITE),
                (mem_slot, abi::rights::WRITE),
            ],
            fault: Some(DEATHS),
            ..supervision_protocol::ChildEndowment::new(supervision_protocol::Retention::Nothing)
        },
    ) else {
        // `memory_region_destroy(region)` is owner authority over the whole region, not the supervised
        // reap `collect_grant` uses later: it reclaims `mem_slot` along with everything else here,
        // because nothing has been handed to a child yet for anyone else to still be holding.
        supervision_protocol::memory_region_destroy(region);
        return None;
    };
    if !supervision_protocol::start_child(child, 0, arg, 0) {
        supervision_protocol::memory_region_destroy(region);
        return None;
    }
    cap_delete(region);
    Some(mem_slot)
}

/// **Wait for the one instance [`fire_with_grant`] just started, and reclaim its grant.**
///
/// Callable only while that instance is the sole thing outstanding, which the one call site in
/// `_start` guarantees by draining everything else first and firing nothing else until this
/// returns. That is what makes the correlation sound: the next death on [`DEATHS`] cannot be
/// anyone else's, so `mem_slot` is destroyed unconditionally rather than guessed at from a refused
/// reap. Once it is gone, the corpse's region has no live resident left and an ordinary [`reap`]
/// reclaims the rest, the same as [`collect`].
fn collect_grant(exits: &mut u64, faults: &mut u64, mem_slot: u64) {
    let (event, tid, _pc, _addr, _rsvd) = recv_fault(DEATHS);
    if event == abi::fault::EVENT_EXIT {
        *exits += 1;
    } else {
        *faults += 1;
    }
    // The grant is a live resident of the corpse's own region (`regions::destroy_outcome` refuses
    // a region with one), so it has to go before `reap` can succeed at all; nothing here needs to
    // try `reap` first and fail to learn that, because this call is only ever made about the one
    // instance that was built with a nested grant.
    supervision_protocol::memory_region_destroy(mem_slot);
    for _ in 0..REAP_ATTEMPTS {
        if reap(DEATHS, tid) == 0 {
            return;
        }
        yield_now();
    }
    user_mode_runtime::trap()
}

/// **Block until one child dies, then collect it**, counting whether it finished or crashed.
///
/// The two events are counted apart for the reason DECISIONS §26 delivers both: a crash is a fact
/// about a scheduled job worth reporting, and a clean exit is the normal end of one. A scheduler
/// that reported only a total would hide exactly the number an operator wants.
///
/// The kernel is the only sender on this endpoint (§26 clears the child's fault slot at `START`), so
/// the tid is trustworthy without a badge.
fn collect(exits: &mut u64, faults: &mut u64) {
    let (event, tid, _pc, _addr, _rsvd) = recv_fault(DEATHS);
    if event == abi::fault::EVENT_EXIT {
        *exits += 1;
    } else {
        *faults += 1;
    }
    for _ in 0..REAP_ATTEMPTS {
        if reap(DEATHS, tid) == 0 {
            return;
        }
        yield_now();
    }
    user_mode_runtime::trap()
}

/// Write bytes down the output endpoint, `byte_sink_protocol`-framed.
fn say(bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(OUT, w0, w1, w2);
        rest = &rest[n..];
    }
}

/// A decimal number down the same stream.
fn say_num(v: u64) {
    let mut digits = [0u8; 20];
    let mut n = 0;
    let mut v = v;
    loop {
        digits[n] = b'0' + (v % 10) as u8;
        v /= 10;
        n += 1;
        if v == 0 {
            break;
        }
    }
    let mut out = [0u8; 20];
    for i in 0..n {
        out[i] = digits[n - 1 - i];
    }
    say(&out[..n]);
}

/// End the stream, report the verdict, and stop.
///
/// The `byte_sink_protocol` end-of-stream comes first so a reader draining text sees a stream that ended
/// rather than one that stopped, and the verdict word after it so a spawn site reading one word
/// still learns how this went.
fn done(code: u64) -> ! {
    send(OUT, byte_sink_protocol::eof(), 0, 0);
    send(OUT, code, 0, 0);
    exit();
}

user_mode_runtime::panic_handler!();
