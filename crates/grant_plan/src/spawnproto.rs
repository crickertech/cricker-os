//! **The spawn protocol: the wire half of the shell-to-progenitor grant expression.**
//!
//! When the shell resolves a `run` into an [`Endowment`](crate::Endowment), it does not build the
//! child itself: the progenitor holds the initrd and is the ELF loader (the parser stays in one place, out of
//! the shell). So the shell tells the progenitor what to spawn and, crucially, *delegates the capabilities it
//! grants* over the same endpoint. This module is that contract's word layout, the capability-shell
//! analogue of `line_editor::proto`.
//!
//! It is a **userspace** protocol, not kernel ABI. The kernel routes these words the way it routes
//! any IPC (DECISIONS §10, §12, §21); it never reads them. Adding a field is a change here, not to
//! the syscall surface.
//!
//! # The exchange
//!
//! The shell owns the sequence; the progenitor serves it in a loop.
//!
//! 1. **Request.** The shell `SEND`s three words on the spawn endpoint: the program id, the
//!    integer argument, and the memory-grant page count. See [`request`] / [`prog_id`] /
//!    [`arg`] / [`mem_pages`].
//! 2. **The directory grant, if the request announced one** ([`Wiring::dir`], milestone 31 phase 3):
//!    [`GRANT_WORDS`] plain `SEND`s carrying the caretaker's `START` words and then the child's.
//!    Before the delegation rather than after, because these are **data and not capabilities** and
//!    mixing the two orders would put a `RECV` where a `RECV_CAP` belongs.
//! 3. **Delegation.** The capabilities the request announced, in a fixed order: the supervised
//!    job's pair (untyped, frame), then the **sink** (milestone 50), then the **source**, then the
//!    **diagnostic endpoint** (DECISIONS §67), then the **screen-narrowed tail's completion
//!    endpoint** (DECISIONS §106), then the `--mem` untyped. Order rather than tags, because both
//!    sides read the same [`Wiring`] out of the same word and a promise nobody receives would
//!    deadlock both.
//!
//!    If `mem_pages > 0`, the shell `SEND_CAP`s exactly one capability there: an
//!    untyped it split from *its own* budget, sized to `mem_pages`. This is the grant made real,
//!    not parsed and dropped. Programs that grant no capability (`least_authority_demo`) skip this step, and the progenitor
//!    knows to skip the matching `RECV_CAP` from `mem_pages == 0`.
//! 4. **Outcome.** The progenitor builds the child, endows it (the shared result endpoint always; the
//!    delegated untyped when present), and starts it. The child reports its own answer on the
//!    result endpoint. If the progenitor cannot build it (its own budget is spent, or the program vanished),
//!    it sends [`SPAWN_FAILED`] on the result endpoint so the shell's read completes instead of
//!    hanging.
//!
//! The result endpoint carries both the progenitor's failure sentinel and the child's success answer, and
//! the shell reads exactly once: a well-formed spawn yields the child's word, a failed one yields
//! [`SPAWN_FAILED`]. One reader, one word, no ambiguity.
//!
//! 5. **Death** (milestone 235). A child the kernel killed sends nothing, so neither of those two
//!    words arrives and the shell's single read has nothing to complete it. `job_undertaker`, which
//!    already holds the progenitor's supervision endpoint and already collects the corpse, sends
//!    [`JOB_FAULTED`] there instead. It is a third value on the same one-word read rather than a
//!    second channel, because the shell has one thread and can be blocked in exactly one `RECV`;
//!    see [`JOB_FAULTED`] for the two couplings this refused.
//!
//! # BUGS
//!
//! The image request ([`Wiring::image`], DECISIONS §219 (how the shell names an installed program to the spawner) option D) is a first cut, and these are
//! what it does not do yet. Milestone 198 (a package manager)'s block carries the same list.
//!
//! - **A caller that claims the run-unvouched capability without holding it hangs the progenitor.**
//!   [`Wiring::run_unvouched`] promises one `SEND` on [`RUN_UNVOUCHED_SLOT`], and the progenitor
//!   waits for it on the endpoint behind that slot, which only a holder can reach. A shell that
//!   sets the bit and holds nothing leaves the progenitor blocked there for the life of the boot.
//!   It is the exposure every promised message in this protocol already has (the frame entry
//!   below), and it is reachable only by a program that holds the spawn endpoint and lies on it,
//!   which today is the boot shell alone.
//! - **A holder can lend its presentation.** The progenitor serves requests one at a time, so a
//!   holder's `SEND` is taken only while some request that claimed the capability is being served.
//!   A holder that sends one early, on purpose, can let another caller's claim through. That is a
//!   proxy, and no capability system prevents a holder from running something on another's behalf;
//!   it is recorded so nobody reads the missing `GRANT` as a stronger promise than it is.
//! - **An image's manifest reaches only what the request words and the progenitor's own grants can
//!   carry** (milestone 597 (a program carries its manifest in an ELF note), provisional). The
//!   manifest now travels in the bytes, as an ELF note (`crates/manifest_note`, §197 (a package is
//!   one archive file) option M2), and [`crate::image_manifest`] endows from it. But this wire has
//!   an argument, a `--mem` count and nothing else a line designates, so a note declaring a file, a
//!   directory, an input, an option, a supervised job, a second stream, a silent output or the
//!   `std` runtime is refused ([`crate::image_can_carry`]) rather than run without it. Each is the
//!   wiring a named program already has, not a new idea.
//! - **An image's [`crate::Endowment`] names a stand-in row** ([`crate::IMAGE_ROW`]), because an
//!   endowment names a `Prog` and a file has none. Everything about an image is decided from the
//!   manifest [`crate::image_manifest`] returns; reading the row instead is the foot gun.
//! - **The shell and the progenitor read two copies of the note.** The shell reads the file to bind
//!   the line, then again to send it; the progenitor judges what arrived. A file changed in between
//!   is judged on what arrived, and a request that no longer fits is refused with
//!   [`SPAWN_REFUSED_BY_MANIFEST`] ([`crate::image_request_fits`]), never half-honoured.
//! - **Only a plain line runs an image.** A path in a pipeline or behind a redirection reaches
//!   the planner as a program name and is refused as "no such program", which is true of the name
//!   and says nothing about the bytes. Nothing sets the bit alongside `interruptible` or `dir`, and
//!   the progenitor refuses an interruptible image request if one arrives.
//! - **Every activation verb is open to whoever holds the spawn endpoint**, including
//!   [`Activation::Vouch`], which vouches for any bytes at all. That is the owner's authority by
//!   DECISIONS §221 (the boot prompt is the owner's console), and it is safe only because the boot
//!   prompt is the one holder. A session given a spawn endpoint would be the owner too. Before any
//!   session is, the activation verbs need a presentation of their own, the way
//!   [`RUN_UNVOUCHED_BIT`] has one.
//! - **A shell that cannot retype a frame mid-request hangs the prompt.** The staging region is
//!   sized to the image, so this needs a full capability table in the shell, but if it happens the
//!   progenitor waits for a frame that never comes. Nothing in the ABI lets either side abandon a
//!   half-sent request; every capability this protocol promises has the same exposure.

/// The interruptible bit, packed into the high half of the page-count word so one `SEND` still
/// carries the whole request. `mem_pages` is a small count (`memory_grant_depleter`'s ceiling is
/// 64), so the low 32 bits hold it and this bit rides above.
const INTERRUPTIBLE_BIT: u64 = 1 << 32;

/// **A capability for the child's output slot follows** (milestone 50). Set by `>` and by every
/// stage of a `|` but the last: the shell delegates an endpoint and the progenitor puts it where the result
/// endpoint would have gone, so the child writes to a pipe or a file sink without knowing which.
const SINK_BIT: u64 = 1 << 33;

/// **A capability for the child's input slot follows** (milestone 50). Set by `<` and by every
/// stage of a `|` but the first.
const SOURCE_BIT: u64 = 1 << 34;

/// **A capability for the child's declared second output follows** (DECISIONS §67). Set for a
/// program whose manifest declares one, whether or not the line has a `2>` on it: the stream exists
/// because the program says so, and the operator only names where it goes.
///
/// Unlike [`SINK_BIT`] this does **not** say which slot: the progenitor reads that from the manifest, because
/// the slot is the program's declaration and not the shell's choice. What the wire says is only
/// "expect one more capability", which is what keeps the two sides in lockstep.
const DIAG_BIT: u64 = 1 << 35;

/// **A capability for a narrowed tail stage's completion signal follows** (DECISIONS §106). Set
/// when the shell decided this stage both writes and reads, and the line named neither `>` nor `|`
/// for its output (`Wiring::sink` is false and the plan says [`crate::line::Sink::Report`]): the
/// shell cannot be both this stage's feeder and its reader, so its primary output defaults to
/// `terminal_sink_caretaker` instead of the shell's own result endpoint, the same adapter a
/// declared second stream already reaches by default under DECISIONS §67.
///
/// What follows is not a sink capability (the progenitor already knows to build that default from its own
/// `term_sink`, unprompted, the same way it builds a diagnostic default). It is a **fresh
/// endpoint the shell minted and kept a copy of**, delegated so the progenitor can install it as this child's
/// DECISIONS §26 fault target in place of its own domain channel. The kernel then delivers the
/// child's exit there instead of to the progenitor's reaper, and the shell `RECV`s it as its completion
/// signal instead of draining the child's bytes, which it no longer sees.
const SCREEN_BIT: u64 = 1 << 37;

/// **A directory grant follows, and the progenitor is to build a caretaker for it** (milestone 31 phase 3).
///
/// The odd one out on this word, because it announces **data rather than a capability**. Every other
/// bit here says "expect one more `SEND_CAP`"; this one says "expect two more `SEND`s", and the
/// reason is that the shell has nothing to delegate. A directory grant is delivered by a
/// `fs_subtree_caretaker`, the caretaker has to hold the file service to attenuate it, and **the
/// shell's file-service endpoint carries no `GRANT`**, so the shell could not hand one over if it
/// wanted to. What it can do is say what the grant *is*; the progenitor holds the endpoint and builds the rest.
///
/// See [`GRANT_WORDS`] for what the two messages carry and why they are opaque to this module.
const DIR_BIT: u64 = 1 << 36;

/// **A second directory grant follows, for the same confined program** (milestone 154,
/// design/roadmap/154-multi-directory-namespace.md). Set only alongside [`DIR_BIT`]: a second
/// grant is meaningless without a first, the same way the kernel's `fs_service::TwoDirGrant`
/// (slot 0 grant A, slot 1 grant B) only exists in pairs.
///
/// **Following [`DIR_BIT`]'s own precedent rather than inventing a new shape**, as milestone
/// 154's roadmap block names as the open question this closes: another two [`GRANT_WORDS`]
/// messages follow the first pair, carrying the second caretaker's `START` words. Unlike
/// [`DIR_BIT`], the confined program's own `START` words are **not** repeated a second time: one
/// program is still being started, holding two narrowed endpoints (slot 0 grant A, slot 1 grant
/// B), the same delivery `fs_service::start_granted_two_dirs` already proved.
///
/// **Nothing on the shell side sets this bit yet.** No verb in `grant_plan` constructs a
/// two-directory `Endowment`: that is milestone 47's `bind`, still unbuilt. This is the wire
/// format and the progenitor's decode side, built ahead of an emitter the way [`DIR_BIT`] itself once
/// stated a grant nothing could construct yet.
const DIR2_BIT: u64 = 1 << 38;

/// **The executable's bytes follow, as page frames the caller owns** (DECISIONS §219 option D,
/// ruled by calef 2026-09-26). Set when the shell runs a program the image did not name by
/// [`Prog`](crate::Prog) id: an installed package's member, or a binary somebody just built.
///
/// **Word 0 changes meaning under this bit**: it carries the image's **length in bytes** instead
/// of a program id ([`image_len`]), and [`image_pages`] of that many `SEND_CAP`s follow, one frame
/// each, in page order, each tagged with its index. They come after the directory grant's data
/// messages and before every other delegated capability, so the data-before-capabilities order the
/// rest of this word keeps is kept here too. The shell sends each frame narrowed to `READ`, so the
/// progenitor can map it and can neither write it nor pass it on.
///
/// **The progenitor hashes its own copy**, never the caller's frames, because the caller keeps a
/// mapping of them and could change the bytes between a hash and a build. It maps each frame
/// through the loader's never-reused scratch window, copies it into a page of its own, and deletes
/// the capability before taking the next, so a request of any size costs its capability table one
/// transient slot. A digest found in the activation set is vouched and runs with that entry's
/// manifest. A miss runs only for a caller that presents the run-unvouched capability
/// ([`RUN_UNVOUCHED_BIT`], §219's gate D2), with [`crate::UNVOUCHED_MANIFEST`]; any other miss is
/// refused with [`SPAWN_UNVOUCHED`].
///
/// Name: provisional (milestone 198 rung 3a, 2026-09-26), §219's own word for it.
const IMAGE_BIT: u64 = 1 << 39;

/// **Not a spawn: an edit to the activation set** (milestone 198 (a package manager) rung 3a's
/// installer; DECISIONS §208 (installing a package is granting it, and the activation set is
/// versioned)). When this bit is set the request asks the progenitor to install, remove or roll
/// back, word 1 is an [`Activation`] verb, the rest of word 2 is zero, and the progenitor answers
/// with exactly one message on the result endpoint: [`activation_reply`]'s two words.
///
/// **Why the progenitor serves it**, rather than an installer program: §208's argument for A3 was
/// that "the supervisor that performs a live swap and the thing that decides which version is
/// active are the same authority", and §219 already gave the progenitor the read half (it looks
/// every image's digest up in the live generation). It holds the file service with `WRITE`, the
/// image's catalogue in its archive, and the frame-staging path an image request built. An
/// installer *program* would need all three delegated to it and an argument vector to be told
/// which package, which does not exist (milestone 205 (how a foreign program is told what to do)).
///
/// What follows the request depends on the verb:
///
/// - For [`Activation::Install`], word 0 is the package file's length in bytes, and its frames follow
///   exactly as an image's do (`IMAGE_BIT`: [`image_pages`] `SEND_CAP`s, each narrowed to `READ`).
///   The progenitor copies them before it hashes, for the image path's reason.
/// - For [`Activation::Remove`], one data message follows, the program's name packed the way a
///   directory grant packs one (`filesystem_protocol::grant::pack_name`: two words, then the
///   length). Opaque here for [`GRANT_WORDS`]'s reason.
/// - For [`Activation::Rollback`], nothing follows.
/// - For [`Activation::Fetch`], the package's name follows as one data message, packed as
///   [`Activation::Remove`]'s program name is. The progenitor finds the name's stem in the image's
///   catalogue, fetches `<stem>.nifepkg` over the network stack it built at boot, and installs
///   what arrived exactly as [`Activation::Install`] installs a file's bytes.
/// - For [`Activation::Vouch`], word 0 is the executable's length, its frames follow as an
///   install's do, and then the name it will be recorded under, as one data message packed as
///   [`Activation::Remove`]'s is. Frames before the name, because the frames are capabilities and
///   the name is data, and the progenitor stages the one before it reads the other.
///
/// Name: provisional (2026-09-26).
const ACTIVATION_BIT: u64 = 1 << 40;

/// **The caller presents the run-unvouched capability: one `SEND` on it follows the delegation**
/// (DECISIONS §219 gate D2, ruled by calef 2026-09-26). Meaningful with [`IMAGE_BIT`]; the
/// progenitor honours it on any request, so a caller that sets it is never left blocked in its
/// `SEND`.
///
/// **Why a message on a second endpoint rather than a capability on this one.** The capability is
/// granted without `GRANT`, so its holder cannot pass it on (§219 limitation 2), and for the same
/// reason it cannot ride a `SEND_CAP` here: the kernel refuses to delegate a capability that lacks
/// `GRANT`. What a holder *can* do with a `WRITE`-only endpoint is send on it. The progenitor holds
/// the only `READ` on that endpoint and takes one message from it, at a fixed point in the
/// exchange, while serving a request that claimed it. A message that arrives there came from a
/// holder, because nothing else can reach the endpoint.
///
/// **Where in the exchange**: after every delegated capability, the last thing a request sends. The
/// progenitor then knows the digest's verdict and the whole delegation before it decides. The word
/// sent is not read: the fact that it arrived is the whole of what it says.
///
/// The progenitor rather than the kernel decides what it means, which is why this is a bit on a
/// userspace word and not a method: DECISIONS §10 (process model: capability-based, microkernel)'s surface is unchanged. The kernel only routes a
/// `SEND` on an endpoint, as it routes every other.
///
/// Name: provisional (milestone 198 rung 3a, 2026-09-26).
const RUN_UNVOUCHED_BIT: u64 = 1 << 41;

/// **Where a session holds the run-unvouched capability** (DECISIONS §219 gate D2): the slot the
/// progenitor places it in, `WRITE` only, in the boot shell and in `login`, and the slot `login`
/// delegates it from.
///
/// Twenty-two, the highest slot below the kernel's reserved fault slot (`abi::fault::FAULT_EP_SLOT`,
/// 23; `grant_plan` does not depend on `abi`, so each binary that reads this asserts the relation
/// itself). A named slot for the reason [`crate::NETWORK_SLOT`] is one: the holder probes it rather
/// than being told, and the probe is sound only at `_start`, before the process has allocated
/// anything, because a runtime allocation takes the first free slot and could land here only in a
/// table that is almost full.
///
/// Name: provisional.
pub const RUN_UNVOUCHED_SLOT: u64 = 22;

/// **What an activation request asks for** (see `ACTIVATION_BIT`). Provisional names, like the
/// bit's; the prompt spells them `package install`, `package remove` and `package rollback`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Activation {
    /// Install the program of the package whose bytes follow: record its digest in a new
    /// generation, place its bytes where a person can run them, and make that generation live.
    Install = 1,
    /// Write a new generation without the named program and make it live. Its bytes stay on
    /// disk, which is what lets a rollback bring it back.
    Remove = 2,
    /// Make the generation numbered one below the live one live again. Nothing is rewritten.
    Rollback = 3,
    /// **Fetch the named package and install it** (milestone 198 rung 3a's fetch): the progenitor
    /// reads the package from the package source over the network, not from the caller, so no
    /// frames follow, only the name. Everything after the bytes arrive is [`Activation::Install`].
    ///
    /// The progenitor rather than a fetching program for the reason the installer is: a program
    /// would need an argument vector to be told which package (milestone 205 (how a foreign
    /// program is told what to do)). The cost is an HTTP reader (`http_response`) in the
    /// progenitor; notes/packages.md weighs it.
    Fetch = 4,
    /// **The owner vouches for these bytes** (DECISIONS §221 (the boot prompt is the owner's
    /// console), ruling 1, and §195 (a reviewed recipe vouches for a package) clause 3: the owner
    /// may vouch for a digest no source carries). The progenitor hashes its own copy and writes a
    /// new generation whose entry for the name is that digest, marked
    /// `activation_set::OWNER` in place of a package, so
    /// `package rollback` undoes it like any other edit. The bytes are not copied: a digest vouches
    /// for itself wherever the file is. They must parse as an executable, or nothing is written.
    ///
    /// **Who may ask** is whoever holds the spawn endpoint, which is the boot prompt and nothing
    /// else; §221 ruled that whoever holds that prompt is the owner. A session `login` builds holds
    /// no spawn endpoint and cannot reach its root to write `activation/` by hand either. The day a
    /// session is given a spawn endpoint, this verb (and install, remove and rollback with it) needs
    /// an authority of its own; the BUGS above say so.
    ///
    /// Name: provisional (milestone 198, lane `milestone/198-owner-console`, 2026-09-26).
    Vouch = 5,
}

impl Activation {
    fn from_word(w: u64) -> Option<Self> {
        match w {
            1 => Some(Self::Install),
            2 => Some(Self::Remove),
            3 => Some(Self::Rollback),
            4 => Some(Self::Fetch),
            5 => Some(Self::Vouch),
            _ => None,
        }
    }
}

/// Build an activation request. `len` is the file's length for [`Activation::Install`] and
/// [`Activation::Vouch`], whose frames follow, and ignored otherwise.
pub fn activation_request(verb: Activation, len: u64) -> (u64, u64, u64) {
    let w0 = if matches!(verb, Activation::Install | Activation::Vouch) {
        len
    } else {
        0
    };
    (w0, verb as u64, ACTIVATION_BIT)
}

/// **The verb, if this request is an activation request at all.** The progenitor asks this first,
/// before [`wiring`], because under `ACTIVATION_BIT` no other bit of word 2 means anything. A set
/// bit with a verb this side does not know is `Some(None)`: the caller answers
/// [`ActivationStatus::Unknown`] and reads nothing more.
pub fn activation(w1: u64, w2: u64) -> Option<Option<Activation>> {
    (w2 & ACTIVATION_BIT != 0).then(|| Activation::from_word(w1))
}

/// **How an activation request came out**: word 0 of the progenitor's one reply. Word 1 is the
/// generation that is live afterwards, whatever the status, so a refusal still says what is in
/// force. Provisional, like the verbs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActivationStatus {
    /// Done: word 1 is the generation now live.
    Done = 0,
    /// The image's catalogue does not vouch for these bytes (DECISIONS §195 (a reviewed recipe vouches for a package)), or they are not a
    /// package at all. Nothing was written.
    NotCatalogued = 1,
    /// A vouched package with no program the installer can find. Nothing was written.
    NoProgram = 2,
    /// [`Activation::Remove`] named a program the live generation does not have.
    NotInstalled = 3,
    /// [`Activation::Rollback`] from a generation with none below it.
    NoEarlier = 4,
    /// The file service refused a write, or this boot has none. What was written before the
    /// failure is described in `notes/packages.md`: never a `current` naming a generation that
    /// was not written whole.
    StoreFailed = 5,
    /// A verb this progenitor does not know, or a package larger than [`IMAGE_MAX_PAGES`].
    Unknown = 6,
    /// [`Activation::Fetch`] named a package the image's catalogue has no line for on this
    /// architecture. Nothing was fetched: the catalogue is asked before the network is.
    NoSuchPackage = 7,
    /// [`Activation::Fetch`] on a boot whose progenitor built no network stack.
    NoNetwork = 8,
    /// [`Activation::Fetch`] could not get a whole package from the source: no connection, a
    /// status other than 200, a response `http_response` refuses, a truncated body, or one larger
    /// than [`IMAGE_MAX_PAGES`]. Nothing was installed.
    FetchFailed = 9,
    /// [`Activation::Vouch`] was sent bytes that do not parse as an executable, or a name the
    /// activation set cannot record. Nothing was written.
    NotExecutable = 10,
}

impl ActivationStatus {
    /// The status a reply's word 0 carries, or [`ActivationStatus::Unknown`] for a word that is
    /// none of them.
    pub fn from_word(w: u64) -> Self {
        match w {
            0 => Self::Done,
            1 => Self::NotCatalogued,
            2 => Self::NoProgram,
            3 => Self::NotInstalled,
            4 => Self::NoEarlier,
            5 => Self::StoreFailed,
            7 => Self::NoSuchPackage,
            8 => Self::NoNetwork,
            9 => Self::FetchFailed,
            10 => Self::NotExecutable,
            _ => Self::Unknown,
        }
    }
}

/// The progenitor's one reply to an activation request: the status and the generation live
/// afterwards (0 when there is none).
pub fn activation_reply(status: ActivationStatus, live: u32) -> (u64, u64, u64) {
    (status as u64, u64::from(live), 0)
}

/// **The largest image `IMAGE_BIT` may carry, in pages** (256 KiB). A ceiling both sides read,
/// so the shell refuses a larger file before it sends anything and the progenitor never stages more
/// than its job pool can hold beside the child built from it. `uptime` is 22 pages stripped.
///
/// Provisional, like the bit: the number is the job pool's arithmetic (`JOBS_BUDGET_PAGES` in
/// `crates/system_initializer`), not a property of any program.
pub const IMAGE_MAX_PAGES: u64 = 64;

/// The page size an image is carried in. A frame is one page on every architecture this tree
/// builds for.
pub const IMAGE_PAGE: u64 = 4096;

/// **The two messages a [`Wiring::dir`] request is followed by**, in order, each three words:
///
/// 1. **the caretaker's `START` words**, which the progenitor passes to `fs_subtree_caretaker` verbatim: the
///    granted directory's name and the `filesystem_protocol::dir` rights the subtree capability is to carry;
/// 2. **the confined program's `START` words**, which the progenitor passes to the program verbatim: for `rm`,
///    the operand's name and the options that were typed.
///
/// **This module does not decode either, deliberately.** They are `filesystem_protocol::grant`'s packing, and
/// `grant_plan` has no non-dev dependency on `filesystem_protocol` on purpose (its own manifest says why: the
/// shell must be able to check a command line without linking the filesystem contract). Passing them
/// through as opaque triples keeps that true, and it means a change to how a grant is packed is a
/// change in one crate rather than in the wire this one owns. The shell packs them; the progenitor forwards
/// them; nothing in between reads them.
///
/// Two messages rather than one because the two processes are started with different names: the
/// caretaker with the *directory*, the program with the *operand inside it*. Six words do not fit in
/// three.
pub const GRANT_WORDS: usize = 2;

/// **Where the shell's operators end up on the wire**, alongside the parts of the endowment that
/// were always here. `sink` and `source` are booleans rather than capabilities because the
/// capability travels separately, over `SEND_CAP`: this word only says whether to expect it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Wiring {
    /// A foreground job the shell will supervise (DECISIONS §24). Two capabilities lead the
    /// delegation: a job untyped the child is built from so the shell can tear it down, then a
    /// shared job frame.
    pub interruptible: bool,
    /// The child's output slot is substituted (`>` or the left of a `|`).
    pub sink: bool,
    /// The child's input slot is filled (`<` or the right of a `|`).
    pub source: bool,
    /// The child declares a second output stream, so one more endpoint follows (DECISIONS §67).
    pub diagnostics: bool,
    /// **A directory grant follows as two data messages** ([`GRANT_WORDS`]), and the progenitor is to build a
    /// `fs_subtree_caretaker` for it before it builds the child. The only entry here that announces
    /// data instead of a capability; see `DIR_BIT`.
    pub dir: bool,
    /// **A second directory grant follows, as two more data messages** (milestone 154). See
    /// `DIR2_BIT`. Meaningless unless `dir` is also set; nothing here enforces that on its own
    /// (the wire is one word, and a caller that set this without `dir` sent a request that was
    /// wrong before it left the shell).
    pub dir2: bool,
    /// **This stage's primary output defaults to `terminal_sink_caretaker`, and a fresh completion
    /// endpoint follows** (DECISIONS §106). See `SCREEN_BIT`. Mutually exclusive with `sink` in
    /// practice (a stage the shell delegated an explicit sink for has somewhere else to write), but
    /// nothing here enforces that; the two ride independent bits because both sides read one word.
    pub screen: bool,
    /// **The executable's bytes follow as frames, and word 0 is their length** (DECISIONS §219
    /// option D). See `IMAGE_BIT`.
    pub image: bool,
    /// **One `SEND` on the run-unvouched capability follows the delegation** (DECISIONS §219 gate
    /// D2). See `RUN_UNVOUCHED_BIT`.
    pub run_unvouched: bool,
}

/// Build the three request words from a resolved endowment's parts.
pub fn request(prog_id: u64, arg: u64, mem_pages: u64, w: Wiring) -> (u64, u64, u64) {
    let mut w2 = mem_pages & 0xffff_ffff;
    if w.interruptible {
        w2 |= INTERRUPTIBLE_BIT;
    }
    if w.sink {
        w2 |= SINK_BIT;
    }
    if w.source {
        w2 |= SOURCE_BIT;
    }
    if w.diagnostics {
        w2 |= DIAG_BIT;
    }
    if w.dir {
        w2 |= DIR_BIT;
    }
    if w.dir2 {
        w2 |= DIR2_BIT;
    }
    if w.screen {
        w2 |= SCREEN_BIT;
    }
    if w.image {
        w2 |= IMAGE_BIT;
    }
    if w.run_unvouched {
        w2 |= RUN_UNVOUCHED_BIT;
    }
    (prog_id, arg, w2)
}

/// The whole wiring of a received request (word 2), so the progenitor reads it once rather than asking three
/// separate questions of the same word.
pub fn wiring(w2: u64) -> Wiring {
    Wiring {
        interruptible: is_interruptible(w2),
        sink: w2 & SINK_BIT != 0,
        source: w2 & SOURCE_BIT != 0,
        diagnostics: w2 & DIAG_BIT != 0,
        dir: w2 & DIR_BIT != 0,
        dir2: w2 & DIR2_BIT != 0,
        screen: w2 & SCREEN_BIT != 0,
        image: w2 & IMAGE_BIT != 0,
        run_unvouched: w2 & RUN_UNVOUCHED_BIT != 0,
    }
}

/// The image's length in bytes from a received request (word 0), when [`Wiring::image`] is set.
/// The same word [`prog_id`] reads otherwise; which one it is depends on word 2 alone.
pub fn image_len(w0: u64) -> u64 {
    w0
}

/// How many frames an image of `len` bytes travels in: one per started page. `0` for an empty
/// image, which the progenitor refuses rather than builds.
pub fn image_pages(len: u64) -> u64 {
    len.div_ceil(IMAGE_PAGE)
}

/// The program id from a received request (word 0).
pub fn prog_id(w0: u64) -> u64 {
    w0
}

/// The integer argument from a received request (word 1).
pub fn arg(w1: u64) -> u64 {
    w1
}

/// The memory-grant page count from a received request (word 2). Non-zero means one delegated
/// untyped capability follows the interrupt caps (if any) over `SEND_CAP` / `RECV_CAP`.
pub fn mem_pages(w2: u64) -> u64 {
    w2 & 0xffff_ffff
}

/// Whether this is a supervised foreground job (word 2's high bit). When set, the delegation leads
/// with two caps: a job untyped (the progenitor builds the child from it; the shell keeps it to `DESTROY`) and
/// a shared job frame (the cooperative interrupt flag and the child's status).
pub fn is_interruptible(w2: u64) -> bool {
    w2 & INTERRUPTIBLE_BIT != 0
}

/// The data word carried alongside the delegated untyped in the `SEND_CAP`. It is not load-bearing
/// (the progenitor identifies the cap by the protocol position, not the tag), but a fixed marker makes a
/// misrouted message obvious in a trace. Its low bits echo the page count as a cheap cross-check.
pub const CAP_TAG: u64 = 0x6361_705f; // "cap_" little-endian-ish marker

/// The sentinel the progenitor sends on the result endpoint when it could not build the child, so the
/// shell's single read completes with a legible failure rather than blocking forever. Distinct
/// from any answer a real program would report (no phase-1 program returns `u64::MAX`).
pub const SPAWN_FAILED: u64 = u64::MAX;

/// **The word for a job the kernel killed** (milestone 235,
/// design/roadmap/235-a-faulted-job-should-reach-the-prompt.md). Sent on the result endpoint by
/// `job_undertaker`, which is the process already holding the progenitor's supervision endpoint, once it has
/// collected the corpse.
///
/// It exists because a faulted job is the one outcome this protocol could not say. A child that
/// exits non-zero has answered; a child the progenitor could not build gets [`SPAWN_FAILED`]; a child the
/// kernel killed **sends nothing at all**, so the shell's single read had nothing to complete it
/// and the prompt never came back (measured 2026-09-02: `least_authority_demo` patched to trap, and
/// `script/swish-check` reporting "the prompt never came back to take `least_authority_demo 7`").
///
/// # Why the supervisor says it rather than the shell asking or the endpoint carrying it
///
/// DECISIONS §26 (the fault endpoint: thread death becomes a message a supervisor holds) delivers
/// every death to exactly **one** endpoint, so the three couplings milestone 235 named are a choice
/// of who holds that endpoint, and only one of them leaves the ordinary paths alone.
///
/// **The shell asking** loses first. A shell that asks has to decide *when* to ask, and with no
/// non-blocking receive in the ABI (`crates/system_initializer`'s own loop records that it has
/// none) that decision is a poll interval, which is a timeout wearing a different hat: it cannot
/// tell a slow job from a dead one, which is the thing the hang already could not tell.
///
/// **The endpoint carrying the death** loses on the ordinary path. Pointing a job's fault target at
/// the endpoint the shell reads would work for a fault, and §26.3 flows *exits* down the same
/// endpoint too, so every ordinary job would leave a second message on the shell's result endpoint
/// behind its answer and the next command's read would take it. It also moves collection into the
/// shell for every job, and takes every job out of the progenitor's supervision domain, which is what
/// `ps`/`pgrep` read (DECISIONS §106 already records that cost as acceptable for one narrow stage
/// and it is not acceptable for all of them).
///
/// **The supervisor telling** costs one capability and one word. `job_undertaker` already receives
/// the death, already collects the corpse, and its own `BUGS` section already recorded that it "has
/// no way to say anything". This is that sentence answered.
///
/// Distinct from [`SPAWN_FAILED`] because the two are different facts a person needs told apart:
/// nothing ran, versus something ran and died. It sits one below `u64::MAX` for the same reason
/// that one sits at it, and the same caveat applies: no program in this tree answers with either.
///
/// Name: provisional, like everything a lane mints: a word in a protocol is exactly the kind of
/// name calef decides.
pub const JOB_FAULTED: u64 = u64::MAX - 1;

/// **The word for bytes nobody vouched for** (DECISIONS §219, milestone 198 rung 3a). Sent on the
/// result endpoint by the progenitor when an `IMAGE_BIT` request's digest is not in the
/// activation set, in place of [`SPAWN_FAILED`], because "nothing was built" and "it was refused
/// as unvouched" are different facts a person needs told apart: the first is out of memory, the
/// second is a decision.
///
/// A miss gets it unless the caller presented the run-unvouched capability ([`Wiring::run_unvouched`],
/// §219's gate D2). A presented miss is built instead, with [`crate::UNVOUCHED_MANIFEST`]: only what
/// the caller delegated, and the clock and configuration pages.
///
/// Two below `u64::MAX`, for [`JOB_FAULTED`]'s reason one below it. Name: provisional.
pub const SPAWN_UNVOUCHED: u64 = u64::MAX - 2;

/// **The progenitor's refusal of bytes whose own manifest forbids the request** (milestone 597,
/// provisional). The note is there and malformed, or it declares something an image request cannot
/// carry yet, or nobody vouched for the bytes and the note asks for what only a command line could
/// designate ([`crate::ImageRefusal`]), or the request's argument or `--mem` count does not fit
/// the manifest the child would be endowed with. Nothing is built.
///
/// A refusal and not a failure, for [`SPAWN_UNVOUCHED`]'s reason: nothing broke. Three below
/// `u64::MAX`, for [`JOB_FAULTED`]'s reason. Name: provisional.
pub const SPAWN_REFUSED_BY_MANIFEST: u64 = u64::MAX - 3;

/// The ack the progenitor sends on the result endpoint when a **supervised** (interruptible) child started
/// cleanly. An interruptible child reports its own progress and exit through the shared job frame,
/// not the result endpoint, so the progenitor sends this once as the go-ahead: the shell reads it, then begins
/// watching the job frame. `0` is distinct from [`SPAWN_FAILED`].
pub const SPAWN_OK: u64 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        // 6, not 1: word zero must come back verbatim, and an id of 1 cannot tell verbatim from a
        // hardcoded answer.
        let (w0, w1, w2) = request(6, 9, 16, Wiring::default());
        assert_eq!(prog_id(w0), 6);
        assert_eq!(arg(w1), 9);
        assert_eq!(mem_pages(w2), 16);
        assert_eq!(wiring(w2), Wiring::default());
    }

    #[test]
    fn interruptible_bit_survives_a_zero_page_count() {
        // The interrupt demonstrators take no --mem, so the flag must ride independent of the count.
        let (_, _, w2) = request(
            2,
            0,
            0,
            Wiring {
                interruptible: true,
                ..Wiring::default()
            },
        );
        assert_eq!(mem_pages(w2), 0);
        assert!(is_interruptible(w2));
    }

    #[test]
    fn no_grant_is_zero_pages() {
        let (_, _, w2) = request(0, 5, 0, Wiring::default());
        assert_eq!(mem_pages(w2), 0);
    }

    /// **The nine flags are independent of each other and of the page count** (milestone 50 (pipes and redirection),
    /// §67 (a program's second stream is a declaration)'s fourth, milestone 31 (a capability shell) phase 3's fifth, DECISIONS §106 (the `terminal_sink_caretaker` narrowing)'s sixth, milestone 154 (a process that holds two directory capabilities)'s
    /// seventh, and §219's image and its gate D2). They share one word, and what the progenitor reads next off the endpoint depends on all of
    /// them, so a bit that bled into another would make the progenitor take a capability for a data word
    /// (or the reverse) and hang rather than fail.
    #[test]
    fn the_wiring_flags_do_not_collide() {
        for &interruptible in &[false, true] {
            for &sink in &[false, true] {
                for &source in &[false, true] {
                    for &diagnostics in &[false, true] {
                        for &dir in &[false, true] {
                            for &dir2 in &[false, true] {
                                for &screen in &[false, true] {
                                    for &image in &[false, true] {
                                        for &run_unvouched in &[false, true] {
                                            let w = Wiring {
                                                interruptible,
                                                sink,
                                                source,
                                                diagnostics,
                                                dir,
                                                dir2,
                                                screen,
                                                image,
                                                run_unvouched,
                                            };
                                            let (_, _, w2) = request(3, 0, 64, w);
                                            assert_eq!(wiring(w2), w, "{w:?}");
                                            assert_eq!(mem_pages(w2), 64, "{w:?}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// **The three sentinels are distinct from each other** (milestone 235). They share one word
    /// on one endpoint and the shell tells them apart by value alone, so a collision would make
    /// "nothing was built", "it ran and died" and "it started" the same message.
    #[test]
    fn the_result_sentinels_do_not_collide() {
        let all = [SPAWN_FAILED, JOB_FAULTED, SPAWN_UNVOUCHED, SPAWN_OK];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    /// **Word 0 is a length under [`IMAGE_BIT`], and it survives the round trip whole** (§219 D).
    /// 89,168 is `uptime`'s stripped size, which is not a page multiple, so the frame count has to
    /// round up: a request that rounded down would send one frame fewer than the progenitor waits
    /// for, and both sides would hang.
    #[test]
    fn an_image_request_carries_its_length_and_rounds_up_to_frames() {
        let (w0, _, w2) = request(
            89_168,
            0,
            0,
            Wiring {
                image: true,
                ..Wiring::default()
            },
        );
        assert!(wiring(w2).image);
        assert_eq!(image_len(w0), 89_168);
        assert_eq!(image_pages(image_len(w0)), 22);
        assert_eq!(image_pages(4096), 1);
        assert_eq!(image_pages(4097), 2);
        assert_eq!(image_pages(0), 0);
    }

    /// **An activation request is told apart from every spawn by one bit, and its verbs and
    /// statuses round-trip** (milestone 198 rung 3a). A spawn request with every wiring flag set is
    /// still not an activation request, which is what keeps the progenitor from reading a spawn's
    /// program id as a package length.
    #[test]
    fn an_activation_request_is_not_a_spawn_and_round_trips() {
        let all = Wiring {
            interruptible: true,
            sink: true,
            source: true,
            diagnostics: true,
            dir: true,
            dir2: true,
            screen: true,
            image: true,
            run_unvouched: true,
        };
        let (_, w1, w2) = request(3, 2, 64, all);
        assert_eq!(activation(w1, w2), None);
        for verb in [
            Activation::Install,
            Activation::Remove,
            Activation::Rollback,
            Activation::Fetch,
            Activation::Vouch,
        ] {
            let (w0, w1, w2) = activation_request(verb, 90_491);
            assert_eq!(activation(w1, w2), Some(Some(verb)));
            assert_eq!(
                w0,
                if matches!(verb, Activation::Install | Activation::Vouch) {
                    90_491
                } else {
                    0
                }
            );
        }
        assert_eq!(activation(9, ACTIVATION_BIT), Some(None));
        for status in [
            ActivationStatus::Done,
            ActivationStatus::NotCatalogued,
            ActivationStatus::NoProgram,
            ActivationStatus::NotInstalled,
            ActivationStatus::NoEarlier,
            ActivationStatus::StoreFailed,
            ActivationStatus::Unknown,
            ActivationStatus::NoSuchPackage,
            ActivationStatus::NoNetwork,
            ActivationStatus::FetchFailed,
            ActivationStatus::NotExecutable,
        ] {
            let (w0, w1, _) = activation_reply(status, 7);
            assert_eq!(ActivationStatus::from_word(w0), status);
            assert_eq!(w1, 7);
        }
    }

    /// **`DIR2_BIT` follows [`DIR_BIT`]'s own precedent**: it is a second bit, not a count, and it
    /// round-trips independent of whether `dir` itself is set (nothing here enforces the "meaningless
    /// without `dir`" rule from the wire alone; that is the emitter's obligation, stated in
    /// [`Wiring::dir2`]'s own doc).
    #[test]
    fn a_second_directory_grant_is_a_second_bit_not_a_count() {
        let (_, _, w2) = request(
            0,
            0,
            0,
            Wiring {
                dir: true,
                dir2: true,
                ..Wiring::default()
            },
        );
        let w = wiring(w2);
        assert!(w.dir);
        assert!(w.dir2);
    }
}
