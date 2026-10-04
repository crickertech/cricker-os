//! **A seeded syscall driver with a shadow model** (milestone 752 (a seeded syscall driver with a
//! shadow model), provisional; part (b) of `design/roadmap/proposals/fuzz-the-surface-a-confined-process-can-reach.md`,
//! ruled yes by calef on 2026-10-04 UTC).
//!
//! From a seed, seven actor threads issue random capability operations through the real syscall
//! layer: `syscall::invoke`, the function the trap handler calls for `SYS_INVOKE`, with slots,
//! methods and arguments drawn at random, including empty and out-of-range slots, methods an object
//! does not have, rights a capability does not carry, revoked frames and both arrival orders at a
//! rendezvous. The conductor (the test thread) keeps a model of every actor's capability table,
//! every rendezvous's queues and the notification's word, predicts each answer before the kernel
//! gives it, and fails on the first difference. After every operation the actor that ran it reads
//! back its whole table, and that is compared slot by slot too, so a capability that arrives where
//! the model says nothing should is a failure even when no register mentions it.
//!
//! **Actor 6 is the witness and holds nothing.** Every operation it issues must be refused as
//! `NoSuchSlot`, and its table must stay empty for the whole run: a thread endowed with nothing
//! gains nothing, whatever the others do around it.
//!
//! **Why the oracle is a model and not a crash.** The three confinement defects of 2026-10-03
//! (#1494, milestone 634 (a plain SEND received by RECEIVE_CAP never hands the receiver a
//! sender-chosen slot), #1525) each delivered something wrong and crashed nothing. A fuzzer that
//! watched only for panics would have passed all three. A kernel panic still ends the run, as it
//! ends any test.
//!
//! **The seed is the reproducer.** Nothing here reads a clock or a core number to decide what to
//! do, so a seed replays the same operations in the same order on every run. A failure prints the
//! seed, the step and the call; `NIFE_SYSCALL_FUZZ_SEEDS=<first>:<count>` at build time (read with
//! `option_env!`) replaces the committed list, which is how the weekly sweep runs many seeds and
//! how one seed is replayed. Coverage-blind on purpose: the proposal records that as an effort
//! trade, not a design preference.
//!
//! **Kernel threads, not an EL0 program, and this is a deviation from the proposal's text.** The
//! proposal said "one EL0 system test program". These actors are kernel threads that hold ordinary
//! capability tables and enter through `syscall::invoke`, which `reap_tests`, `notification_tests`
//! and `timer_tests` already do for the same reason. Two things decided it. The oracle needs to
//! know which side of a rendezvous parked first, and only the kernel can say that without timing
//! guesses (`sched::thread_death_disposition`). And an EL0 helper thread needs a retyped TCB and
//! an address space per helper, which buys the trap entry and nothing the model checks. What this
//! does not exercise is the trap entry itself (register marshalling in `arch`), which every EL0
//! test in the suite already crosses.
//!
//! # What the generator reaches
//!
//! Objects: two rendezvous, one notification, two single-page frames, and the Reply capabilities
//! `CALL` mints. Methods: `SEND`, `RECEIVE`, `SEND_CAP`, `RECEIVE_CAP`, `CALL`, `BADGE` on a
//! rendezvous; `REPLY`; `SIGNAL`, `WAIT`, `POLL`, `BIND` on the notification; `REVOKE` on a frame;
//! an unknown method on each; and `SYS_CAP_DELETE`.
//!
//! # BUGS
//!
//! - **`SYS_CAP_DELETE` is driven through `sched::delete_current_cap`, not `syscall::dispatch`.**
//!   `TrapFrame` has no setter for the syscall number, so a kernel thread cannot build a frame
//!   `dispatch` would read as `SYS_CAP_DELETE`. `dispatch`'s arm is that one call and `Ok(0)`.
//! - **Not generated:** `REAP`, `SURVEY`, `MemoryRegion`, `AddressSpace`, `ThreadControlBlock`,
//!   `Timer`, `Irq`, `PageFrame::MAP` and `SLICE`, a bound notification (no TCB capability exists,
//!   so `BIND` only reaches its refusals), and death messages. Each is a model extension, not a
//!   rewrite; a timer needs the model to reason about time, which is why it was left.
//! - **The model encodes what the kernel does today for a capability delivered to a parked plain
//!   `RECEIVE`**: a `SEND_CAP` or `CALL` that finds a plain `RECEIVE` already parked installs the
//!   capability in the receiver's table and returns its slot in `x1`, while the same pair in the
//!   other order delivers no capability (the fix of milestone 633 (an outside agent attacks the
//!   confinement claim)). The asymmetry is written up for an architect in
//!   `design/roadmap/752-*.md`; until it is ruled on, this file pins the behaviour so a change to
//!   it is seen.
//! - **Unexpected wakes are seen late.** An actor the model says is parked is checked for a stray
//!   answer at every step and at teardown, so a wrong wake is caught, but possibly some steps after
//!   the operation that caused it.

use core::sync::atomic::{AtomicU64, Ordering};

use abi::Error;

use super::wait_for;
use crate::arch::exceptions::TrapFrame;
use crate::cap::{Cap, Object, Rights};
use crate::sched::{self, RendezvousId};
use crate::thread::{State, ThreadId, Wait, WaitRole};

const SLOTS: usize = abi::CAPABILITY_TABLE_SLOTS as usize;
/// Six actors that are endowed, and the witness.
const ACTORS: usize = 7;
const WITNESS: usize = ACTORS - 1;
const EPS: usize = 2;
const FRAMES: usize = 2;
/// Operations per seed. A seed also ends early when every endowed actor is parked.
const STEPS: usize = 96;
/// The committed seeds: `0..SUITE_SEEDS`, each mixed through `splitmix` before use, then [`CORPUS`].
/// Chosen from the measured wall time (see the roadmap block); the weekly sweep runs thousands.
const SUITE_SEEDS: u64 = 32;
/// **Seeds kept because they found something**, syzkaller's corpus in one line. The range above
/// turns red under five of the six falsifications milestone 752 was measured against; seed 180 is
/// the first a 5,000-seed sweep found red under the sixth, the patch for milestone 633 (an outside
/// agent attacks the confinement claim), which needs a delegation left staged by a plain `RECEIVE`
/// and then a later plain `SEND` collected by `RECEIVE_CAP` (2026-10-04, UTC). A seed is tied to
/// this generator: a change to `pick_op` or `endow` re-rolls every seed, and the corpus must be
/// re-found, which the roadmap block's replay table is for. A new finding's seed belongs here with
/// its reason.
const CORPUS: &[u64] = &[180];
/// The sweep's guest-time cap: it stops between seeds once this has passed and says how far it
/// got, so a weekly run with a large count fits under the per-test budget
/// (`testing.rs`' `DEFAULT_BUDGET_SECS`, 90 s).
const SWEEP_SECS: u64 = 60;
const NO_CAP: u64 = abi::rendezvous::NO_CAP;
/// A method number no object answers.
const NO_METHOD: u64 = 99;

// ---------------------------------------------------------------------------------------------
// The generator.

fn splitmix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        splitmix(self.0)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
    /// A data word: mostly small, sometimes a value a receiver could misread (`NO_CAP`, an error
    /// code, the `BOUND` and `REPLY_DELIVERED` tags).
    fn word(&mut self) -> u64 {
        match self.below(8) {
            0 => NO_CAP,
            1 => (Error::NoSuchSlot as i64) as u64,
            2 => abi::notification::BOUND,
            3 => abi::rendezvous::REPLY_DELIVERED,
            4 => self.next(),
            _ => self.below(64),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The model.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Obj {
    Ep(usize),
    Note,
    Frame(usize),
    /// A Reply naming this actor.
    Reply(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct MCap {
    obj: Obj,
    rights: u32,
    badge: u64,
}

impl MCap {
    fn allows(&self, r: Rights) -> bool {
        self.rights & r.bits() == r.bits()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Park {
    Idle,
    Send {
        ep: usize,
        msg: [u64; 3],
        badge: u64,
    },
    SendCap {
        ep: usize,
        data: u64,
        cap: Option<MCap>,
        badge: u64,
    },
    /// Queued as a sender with its Reply staged.
    Call {
        ep: usize,
        msg: [u64; 2],
        badge: u64,
    },
    /// Collected; waiting for a `REPLY` that may never come (a plain `RECEIVE` collected it, or the
    /// server deleted the Reply). Only teardown ends that.
    AwaitReply,
    Recv {
        ep: usize,
        cap: bool,
    },
    Wait,
}

/// What an actor's call returns: `x0` as the syscall layer's `Result`, and the registers the method
/// writes (`n` of `x1..x4`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Answer {
    r: Result<i64, Error>,
    regs: [u64; 4],
    n: usize,
}

impl Answer {
    const fn ok(v: i64) -> Self {
        Answer {
            r: Ok(v),
            regs: [0; 4],
            n: 0,
        }
    }
    const fn err(e: Error) -> Self {
        Answer {
            r: Err(e),
            regs: [0; 4],
            n: 0,
        }
    }
    /// A receive's five words, as `RECEIVE` and `RECEIVE_CAP` return them.
    fn msg(m: [u64; 5]) -> Self {
        Answer {
            r: Ok(m[0] as i64),
            regs: [m[1], m[2], m[3], m[4]],
            n: 4,
        }
    }
    fn matches(&self, real: &Answer) -> bool {
        self.r == real.r && self.regs[..self.n] == real.regs[..self.n]
    }
}

#[derive(Clone, Copy)]
struct Queue {
    v: [usize; ACTORS],
    n: usize,
}

impl Queue {
    const fn new() -> Self {
        Queue {
            v: [0; ACTORS],
            n: 0,
        }
    }
    fn push(&mut self, a: usize) {
        self.v[self.n] = a;
        self.n += 1;
    }
    fn pop(&mut self) -> Option<usize> {
        if self.n == 0 {
            return None;
        }
        let a = self.v[0];
        self.v.copy_within(1..self.n, 0);
        self.n -= 1;
        Some(a)
    }
}

struct Model {
    tables: [[Option<MCap>; SLOTS]; ACTORS],
    park: [Park; ACTORS],
    senders: [Queue; EPS],
    receivers: [Queue; EPS],
    word: u64,
    waiters: Queue,
    census: Census,
}

/// What a run reached, counted from the model, so a green run can say it went somewhere.
#[derive(Clone, Copy, Default, Debug)]
struct Census {
    calls: u64,
    refused: u64,
    parked: u64,
    delegated: u64,
    replied: u64,
    revoked: u64,
    /// A frame revoked while a copy of it was staged in a parked `SEND_CAP`.
    staged_frame_revoked: u64,
    /// A `SEND_CAP` or `CALL` that found a plain `RECEIVE` parked and installed a capability there.
    cap_to_plain_receive: u64,
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Invoke { slot: u64, method: u64, a: [u64; 3] },
    Delete { slot: u64 },
}

/// The model's verdict on one call: the caller's own answer (or `None`: it parks), and the answers
/// of the parked actors it wakes.
struct Verdict {
    me: Option<Answer>,
    woken: [Option<Answer>; ACTORS],
}

impl Model {
    fn new() -> Self {
        Model {
            tables: [[None; SLOTS]; ACTORS],
            park: [Park::Idle; ACTORS],
            senders: [Queue::new(); EPS],
            receivers: [Queue::new(); EPS],
            word: 0,
            waiters: Queue::new(),
            census: Census::default(),
        }
    }

    fn insert(&mut self, a: usize, c: MCap) -> u64 {
        match self.tables[a].iter().position(Option::is_none) {
            Some(s) => {
                self.tables[a][s] = Some(c);
                s as u64
            }
            None => NO_CAP,
        }
    }

    fn cap(&self, a: usize, slot: u64) -> Option<MCap> {
        if slot < SLOTS as u64 {
            self.tables[a][slot as usize]
        } else {
            None
        }
    }

    fn step(&mut self, a: usize, op: Op) -> Verdict {
        let mut v = Verdict {
            me: None,
            woken: [None; ACTORS],
        };
        match op {
            Op::Delete { slot } => {
                if slot < SLOTS as u64 {
                    self.tables[a][slot as usize] = None;
                }
                v.me = Some(Answer::ok(0));
            }
            Op::Invoke {
                slot,
                method,
                a: args,
            } => {
                let cap = self.cap(a, slot);
                v.me = match cap {
                    None => Some(Answer::err(Error::NoSuchSlot)),
                    Some(cap) => self.invoke(a, slot, cap, method, args, &mut v.woken),
                };
                if matches!(cap, Some(c) if matches!(c.obj, Obj::Ep(_)))
                    && method == abi::rendezvous::SEND_CAP
                    && !matches!(v.me, Some(Answer { r: Err(_), .. }))
                {
                    self.census.delegated += 1;
                }
            }
        }
        self.census.calls += 1;
        match v.me {
            None => self.census.parked += 1,
            Some(Answer { r: Err(_), .. }) => self.census.refused += 1,
            Some(_) => {}
        }
        v
    }

    fn invoke(
        &mut self,
        a: usize,
        slot: u64,
        cap: MCap,
        method: u64,
        [a0, a1, a2]: [u64; 3],
        woken: &mut [Option<Answer>; ACTORS],
    ) -> Option<Answer> {
        use abi::{notification as n, page_frame as f, rendezvous as r};
        let refuse = |e| Some(Answer::err(e));
        match cap.obj {
            Obj::Ep(ep) => match method {
                r::SEND => {
                    if !cap.allows(Rights::WRITE) {
                        return refuse(Error::NotPermitted);
                    }
                    let msg = [a0, a1, a2, cap.badge, 0];
                    match self.receivers[ep].pop() {
                        Some(rx) => {
                            woken[rx] = Some(self.deliver_plain(rx, msg));
                            Some(Answer::ok(0))
                        }
                        None => {
                            self.senders[ep].push(a);
                            self.park[a] = Park::Send {
                                ep,
                                msg: [a0, a1, a2],
                                badge: cap.badge,
                            };
                            None
                        }
                    }
                }
                r::RECEIVE | r::RECEIVE_CAP => {
                    if !cap.allows(Rights::READ) {
                        return refuse(Error::NotPermitted);
                    }
                    let with_cap = method == r::RECEIVE_CAP;
                    let Some(tx) = self.senders[ep].pop() else {
                        self.receivers[ep].push(a);
                        self.park[a] = Park::Recv { ep, cap: with_cap };
                        return None;
                    };
                    Some(self.collect(a, tx, with_cap, woken))
                }
                r::SEND_CAP => {
                    if !cap.allows(Rights::WRITE) {
                        return refuse(Error::NotPermitted);
                    }
                    let Some(src) = self.cap(a, a0) else {
                        return refuse(Error::NoSuchSlot);
                    };
                    if !src.allows(Rights::GRANT) {
                        return refuse(Error::NotPermitted);
                    }
                    let narrowed = (a1 as u32) & Rights::ALL.bits();
                    if narrowed & !src.rights != 0 {
                        return refuse(Error::NotPermitted);
                    }
                    let delegated = MCap {
                        obj: src.obj,
                        rights: narrowed,
                        badge: src.badge,
                    };
                    match self.receivers[ep].pop() {
                        // Receiver-first: the capability is installed whichever kind of receive
                        // parked (see this module's BUGS).
                        Some(rx) => {
                            let s = self.insert(rx, delegated);
                            if matches!(self.park[rx], Park::Recv { cap: false, .. }) && s != NO_CAP
                            {
                                self.census.cap_to_plain_receive += 1;
                            }
                            self.park[rx] = Park::Idle;
                            woken[rx] = Some(Answer::msg([a2, s, 0, cap.badge, 0]));
                            Some(Answer::ok(0))
                        }
                        None => {
                            self.senders[ep].push(a);
                            self.park[a] = Park::SendCap {
                                ep,
                                data: a2,
                                cap: Some(delegated),
                                badge: cap.badge,
                            };
                            None
                        }
                    }
                }
                r::CALL => {
                    if !cap.allows(Rights::WRITE) {
                        return refuse(Error::NotPermitted);
                    }
                    match self.receivers[ep].pop() {
                        Some(rx) => {
                            let s = self.insert(rx, reply_of(a));
                            if matches!(self.park[rx], Park::Recv { cap: false, .. }) && s != NO_CAP
                            {
                                self.census.cap_to_plain_receive += 1;
                            }
                            self.park[rx] = Park::Idle;
                            woken[rx] = Some(Answer::msg([a0, s, a1, cap.badge, reply_tag(s)]));
                            self.park[a] = Park::AwaitReply;
                        }
                        None => {
                            self.senders[ep].push(a);
                            self.park[a] = Park::Call {
                                ep,
                                msg: [a0, a1],
                                badge: cap.badge,
                            };
                        }
                    }
                    None
                }
                r::BADGE => {
                    if !cap.allows(Rights::GRANT) || a0 == 0 || cap.badge != 0 {
                        return refuse(Error::NotPermitted);
                    }
                    match self.insert(a, MCap { badge: a0, ..cap }) {
                        NO_CAP => refuse(Error::OutOfMemory),
                        s => Some(Answer::ok(s as i64)),
                    }
                }
                _ => refuse(Error::BadMethod),
            },
            Obj::Reply(caller) => match method {
                abi::reply::REPLY => {
                    if !cap.allows(Rights::WRITE) {
                        return refuse(Error::NotPermitted);
                    }
                    if self.park[caller] == Park::AwaitReply {
                        self.census.replied += 1;
                        self.park[caller] = Park::Idle;
                        woken[caller] = Some(Answer {
                            r: Ok(a0 as i64),
                            regs: [a1, 0, 0, 0],
                            n: 1,
                        });
                    }
                    self.tables[a][slot as usize] = None; // one-shot
                    Some(Answer::ok(0))
                }
                _ => refuse(Error::BadMethod),
            },
            Obj::Note => match method {
                n::SIGNAL => {
                    if !cap.allows(Rights::WRITE) {
                        return refuse(Error::NotPermitted);
                    }
                    if a0 != 0 {
                        match self.waiters.pop() {
                            Some(w) => {
                                self.park[w] = Park::Idle;
                                woken[w] = Some(Answer::ok(a0 as i64));
                            }
                            None => self.word |= a0,
                        }
                    }
                    Some(Answer::ok(0))
                }
                n::WAIT => {
                    if !cap.allows(Rights::READ) {
                        return refuse(Error::NotPermitted);
                    }
                    if self.word != 0 {
                        Some(Answer::ok(core::mem::take(&mut self.word) as i64))
                    } else {
                        self.waiters.push(a);
                        self.park[a] = Park::Wait;
                        None
                    }
                }
                n::POLL => {
                    if !cap.allows(Rights::READ) {
                        return refuse(Error::NotPermitted);
                    }
                    Some(Answer::ok(core::mem::take(&mut self.word) as i64))
                }
                n::BIND => {
                    if !cap.allows(Rights::WRITE) {
                        return refuse(Error::NotPermitted);
                    }
                    // No actor holds a thread capability, so BIND reaches only its refusals.
                    match self.cap(a, a0) {
                        None => refuse(Error::NoSuchSlot),
                        Some(_) => refuse(Error::WrongObject),
                    }
                }
                _ => refuse(Error::BadMethod),
            },
            Obj::Frame(fr) => match method {
                f::REVOKE => {
                    if !cap.allows(Rights::GRANT) {
                        return refuse(Error::NotPermitted);
                    }
                    // Every copy in every table goes, the invoker's too, and so does a copy staged
                    // in a parked SEND_CAP (the 2026-09-21 fix).
                    for t in self.tables.iter_mut() {
                        for s in t.iter_mut() {
                            if matches!(s, Some(c) if c.obj == Obj::Frame(fr)) {
                                *s = None;
                            }
                        }
                    }
                    self.census.revoked += 1;
                    for p in self.park.iter_mut() {
                        if let Park::SendCap { cap: c, .. } = p
                            && matches!(c, Some(c) if c.obj == Obj::Frame(fr))
                        {
                            *c = None;
                            self.census.staged_frame_revoked += 1;
                        }
                    }
                    Some(Answer::ok(0))
                }
                _ => refuse(Error::BadMethod),
            },
        }
    }

    /// A plain `SEND` reaching a parked receiver of either kind.
    fn deliver_plain(&mut self, rx: usize, m: [u64; 5]) -> Answer {
        let Park::Recv { cap, .. } = self.park[rx] else {
            unreachable!("a queued receiver is parked receiving")
        };
        self.park[rx] = Park::Idle;
        if cap {
            // Milestone 634: x1 is NO_CAP unless a capability was installed.
            Answer::msg([m[0], NO_CAP, m[1], m[3], 0])
        } else {
            Answer::msg(m)
        }
    }

    /// Receiver `a` collects the parked sender `tx`.
    fn collect(
        &mut self,
        a: usize,
        tx: usize,
        with_cap: bool,
        woken: &mut [Option<Answer>; ACTORS],
    ) -> Answer {
        match self.park[tx] {
            Park::Send { msg, badge, .. } => {
                self.park[tx] = Park::Idle;
                woken[tx] = Some(Answer::ok(0));
                if with_cap {
                    Answer::msg([msg[0], NO_CAP, msg[1], badge, 0])
                } else {
                    Answer::msg([msg[0], msg[1], msg[2], badge, 0])
                }
            }
            Park::SendCap {
                data, cap, badge, ..
            } => {
                self.park[tx] = Park::Idle;
                woken[tx] = Some(Answer::ok(0));
                if with_cap {
                    let s = match cap {
                        Some(c) => self.insert(a, c),
                        None => NO_CAP,
                    };
                    Answer::msg([data, s, 0, badge, 0])
                } else {
                    // Milestone 633: a plain RECEIVE takes the data and drops the staged capability.
                    Answer::msg([data, 0, 0, badge, 0])
                }
            }
            Park::Call { msg, badge, .. } => {
                self.park[tx] = Park::AwaitReply;
                if with_cap {
                    let s = self.insert(a, reply_of(tx));
                    Answer::msg([msg[0], s, msg[1], badge, reply_tag(s)])
                } else {
                    // The caller stays parked with no Reply anywhere; only teardown frees it.
                    Answer::msg([msg[0], msg[1], 0, badge, 0])
                }
            }
            p => unreachable!("a queued sender is parked sending, not {p:?}"),
        }
    }

    /// Teardown: every parked actor is woken `Gone`, and the Replies naming a stranded caller are
    /// swept from every table (`sched::strand_reply_caller`).
    fn teardown(&mut self) -> [Option<Answer>; ACTORS] {
        let mut woken = [None; ACTORS];
        for a in 0..ACTORS {
            if self.park[a] != Park::Idle {
                woken[a] = Some(Answer::err(Error::Gone));
                if matches!(self.park[a], Park::Call { .. } | Park::AwaitReply) {
                    for t in self.tables.iter_mut() {
                        for s in t.iter_mut() {
                            if matches!(s, Some(c) if c.obj == Obj::Reply(a)) {
                                *s = None;
                            }
                        }
                    }
                }
                self.park[a] = Park::Idle;
            }
        }
        woken
    }
}

fn reply_of(caller: usize) -> MCap {
    MCap {
        obj: Obj::Reply(caller),
        rights: Rights::WRITE.bits(),
        badge: 0,
    }
}

fn reply_tag(slot: u64) -> u64 {
    if slot == NO_CAP {
        0
    } else {
        abi::rendezvous::REPLY_DELIVERED
    }
}

// ---------------------------------------------------------------------------------------------
// The actors.

#[derive(Clone, Copy)]
enum Cmd {
    Grant(Cap),
    Op(Op),
    /// Read back the table only.
    Look,
    /// Empty every slot, between seeds.
    Clear,
    Exit,
}

#[derive(Clone, Copy)]
struct Mail {
    seq: u64,
    cmd: Cmd,
    answer: Answer,
    table: [Option<Cap>; SLOTS],
}

static MAIL: [spin::Mutex<Mail>; ACTORS] = [const {
    spin::Mutex::new(Mail {
        seq: 0,
        cmd: Cmd::Look,
        answer: Answer::ok(0),
        table: [None; SLOTS],
    })
}; ACTORS];
static DONE: [AtomicU64; ACTORS] = [const { AtomicU64::new(0) }; ACTORS];

fn actor(i: usize, doorbell: RendezvousId) {
    loop {
        let _ = sched::ipc_receive(doorbell);
        let (seq, cmd) = {
            let m = MAIL[i].lock();
            (m.seq, m.cmd)
        };
        let answer = match cmd {
            Cmd::Exit => {
                DONE[i].store(seq, Ordering::SeqCst);
                return;
            }
            Cmd::Grant(c) => match sched::grant(c) {
                Ok(s) => Answer::ok(s as i64),
                Err(_) => Answer::err(Error::OutOfMemory),
            },
            Cmd::Look => Answer::ok(0),
            Cmd::Clear => {
                for s in 0..SLOTS as u64 {
                    let _ = sched::delete_current_cap(s);
                }
                Answer::ok(0)
            }
            Cmd::Op(Op::Delete { slot }) => {
                // `syscall::dispatch`'s SYS_CAP_DELETE arm, which ignores the result.
                let _ = sched::delete_current_cap(slot);
                Answer::ok(0)
            }
            Cmd::Op(Op::Invoke { slot, method, a }) => {
                let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
                let r = crate::syscall::invoke(&mut frame, slot, method, a[0], a[1], a[2]);
                Answer {
                    r,
                    regs: [frame.arg(1), frame.arg(2), frame.arg(3), frame.arg(4)],
                    n: 4,
                }
            }
        };
        let mut table = [None; SLOTS];
        for (s, t) in table.iter_mut().enumerate() {
            *t = sched::current_cap(s as u64).ok();
        }
        {
            let mut m = MAIL[i].lock();
            m.answer = answer;
            m.table = table;
        }
        DONE[i].store(seq, Ordering::SeqCst);
    }
}

/// The objects one seed runs against, by their kernel names, so a real capability can be read back
/// as a model one.
struct World {
    region: u64,
    eps: [RendezvousId; EPS],
    note: sched::NotificationId,
    frames: [u64; FRAMES],
}

struct Stage {
    tids: [ThreadId; ACTORS],
    doorbells: [RendezvousId; ACTORS],
    seq: u64,
}

impl Stage {
    fn post(&mut self, a: usize, cmd: Cmd) -> u64 {
        self.seq += 1;
        {
            let mut m = MAIL[a].lock();
            m.seq = self.seq;
            m.cmd = cmd;
        }
        sched::ipc_send(self.doorbells[a], [0, 0, 0]);
        self.seq
    }

    fn finished(a: usize, seq: u64) -> bool {
        DONE[a].load(Ordering::SeqCst) == seq
    }

    /// Parked in a fuzzed operation, as opposed to idle on its doorbell.
    fn parked(&self, a: usize) -> bool {
        sched::thread_death_disposition(self.tids[a]).is_some_and(|d| {
            d.state == State::Blocked
                && !matches!(d.wait_on, Some(Wait::Rendezvous(ep, WaitRole::Receiver)) if ep == self.doorbells[a])
        })
    }

    fn answer(a: usize) -> (Answer, [Option<Cap>; SLOTS]) {
        let m = MAIL[a].lock();
        (m.answer, m.table)
    }

    /// Run `cmd` on idle actor `a` and wait for it to finish (`Some`) or park (`None`).
    fn run(&mut self, a: usize, cmd: Cmd) -> Option<(Answer, [Option<Cap>; SLOTS])> {
        let seq = self.post(a, cmd);
        assert!(
            wait_for(|| Self::finished(a, seq) || self.parked(a)),
            "actor {a} neither finished nor parked within the deadline"
        );
        Self::finished(a, seq).then(|| Self::answer(a))
    }
}

fn abstract_cap(c: &Cap, w: &World, tids: &[ThreadId; ACTORS]) -> Option<MCap> {
    let obj = match c.object {
        Object::Rendezvous(id, _) => Obj::Ep(w.eps.iter().position(|&e| e == id)?),
        Object::Notification(id) if id == w.note => Obj::Note,
        Object::PageFrame(phys, n) if n.get() == 1 => {
            Obj::Frame(w.frames.iter().position(|&f| f == phys)?)
        }
        Object::Reply(tid) => Obj::Reply(tids.iter().position(|&t| t == tid)?),
        _ => return None,
    };
    let badge = match c.object {
        Object::Rendezvous(_, b) => b,
        _ => 0,
    };
    Some(MCap {
        obj,
        rights: c.rights.bits(),
        badge,
    })
}

fn real_cap(m: MCap, w: &World) -> Cap {
    let rights = Rights::from_bits(m.rights);
    match m.obj {
        Obj::Ep(e) => crate::cap::rendezvous_cap_badged(w.eps[e], rights, m.badge),
        Obj::Note => crate::cap::notification_cap(w.note, rights),
        Obj::Frame(f) => crate::cap::page_frame_cap(w.frames[f], rights),
        Obj::Reply(_) => unreachable!("a Reply is never endowed"),
    }
}

/// One seed's run. `Err` is the first disagreement, worded for the failure message.
struct Run<'a> {
    seed: u64,
    step: usize,
    model: Model,
    world: World,
    stage: &'a mut Stage,
}

macro_rules! mismatch {
    ($run:expr, $($arg:tt)*) => {
        panic!(
            "syscall fuzzer: seed {:#x} step {}: {} (replay: NIFE_SYSCALL_FUZZ_SEEDS={}:1)",
            $run.seed, $run.step, format_args!($($arg)*), $run.seed
        )
    };
}

impl Run<'_> {
    fn check_table(&self, a: usize, real: &[Option<Cap>; SLOTS], what: &dyn core::fmt::Debug) {
        for s in 0..SLOTS {
            let got = real[s].map(|c| abstract_cap(&c, &self.world, &self.stage.tids));
            let want = self.model.tables[a][s];
            let same = match (got, want) {
                (None, None) => true,
                (Some(Some(g)), Some(w)) => g == w,
                _ => false,
            };
            if !same {
                mismatch!(
                    self,
                    "after {what:?}, actor {a}'s slot {s} holds {:?} and the model says {want:?}",
                    real[s]
                );
            }
        }
    }

    fn check_woken(&mut self, woken: &[Option<Answer>; ACTORS], what: &dyn core::fmt::Debug) {
        for (b, want) in woken.iter().enumerate() {
            let Some(want) = want else { continue };
            let seq = MAIL[b].lock().seq;
            if !wait_for(|| Stage::finished(b, seq)) {
                mismatch!(
                    self,
                    "{what:?} should have woken actor {b} with {want:?}; it stayed parked"
                );
            }
            let (got, table) = Stage::answer(b);
            if !want.matches(&got) {
                mismatch!(
                    self,
                    "{what:?} woke actor {b} with {got:?}; the model says {want:?}"
                );
            }
            self.check_table(b, &table, what);
        }
    }

    /// It parked, and on the queue the model says: the right object, in the right role.
    fn check_parked_where(&self, a: usize, op: Op) {
        let e = |ep: usize| self.world.eps[ep];
        let want = match self.model.park[a] {
            Park::Send { ep, .. } | Park::SendCap { ep, .. } => {
                Some(Wait::Rendezvous(e(ep), WaitRole::Sender))
            }
            Park::Call { ep, .. } => Some(Wait::Rendezvous(e(ep), WaitRole::Reply)),
            Park::Recv { ep, .. } => Some(Wait::Rendezvous(e(ep), WaitRole::Receiver)),
            Park::Wait => Some(Wait::Notification(self.world.note)),
            // Reply-parked on whichever rendezvous it called; the role is what is checkable.
            Park::AwaitReply => None,
            Park::Idle => unreachable!("a parked verdict leaves the actor parked"),
        };
        let got = sched::thread_death_disposition(self.stage.tids[a]).and_then(|d| d.wait_on);
        let ok = match want {
            Some(w) => got == Some(w),
            None => matches!(got, Some(Wait::Rendezvous(_, WaitRole::Reply))),
        };
        if !ok {
            mismatch!(
                self,
                "actor {a} {op:?} parked on {got:?}; the model has it as {:?}",
                self.model.park[a]
            );
        }
    }

    /// No actor the model holds parked may have answered.
    fn check_still_parked(&self) {
        for b in 0..ACTORS {
            if self.model.park[b] != Park::Idle {
                let seq = MAIL[b].lock().seq;
                if Stage::finished(b, seq) {
                    let (got, _) = Stage::answer(b);
                    mismatch!(
                        self,
                        "actor {b} answered {got:?} while the model holds it parked as {:?}",
                        self.model.park[b]
                    );
                }
            }
        }
    }

    fn one(&mut self, a: usize, op: Op) {
        let verdict = self.model.step(a, op);
        let what = (a, op);
        match (self.stage.run(a, Cmd::Op(op)), verdict.me) {
            (Some((got, table)), Some(want)) => {
                if !want.matches(&got) {
                    mismatch!(
                        self,
                        "actor {a} {op:?} answered {got:?}; the model says {want:?}"
                    );
                }
                self.check_table(a, &table, &what);
            }
            (Some((got, _)), None) => mismatch!(
                self,
                "actor {a} {op:?} answered {got:?}; the model says it parks as {:?}",
                self.model.park[a]
            ),
            (None, Some(want)) => {
                mismatch!(
                    self,
                    "actor {a} {op:?} parked; the model says it answers {want:?}"
                )
            }
            (None, None) => self.check_parked_where(a, op),
        }
        self.check_woken(&verdict.woken, &what);
        self.check_still_parked();
    }

    fn teardown(&mut self) {
        // Every idle actor's table, once more, before reclaiming the objects it names.
        for a in 0..ACTORS {
            if self.model.park[a] == Park::Idle {
                let (_, table) = self
                    .stage
                    .run(a, Cmd::Look)
                    .expect("an idle actor parked on a look");
                self.check_table(a, &table, &"the look before teardown");
            }
        }
        let woken = self.model.teardown();
        let region = self.world.region;
        assert!(
            wait_for(|| sched::reclaim_region(region).is_ok()),
            "seed {:#x}: the fuzz region could not be reclaimed",
            self.seed
        );
        // Frames go with their region; the model forgets them so the comparison is about the rest.
        for t in self.model.tables.iter_mut() {
            for s in t.iter_mut() {
                if matches!(s, Some(c) if matches!(c.obj, Obj::Frame(_))) {
                    *s = None;
                }
            }
        }
        self.check_woken_after_teardown(&woken);
        for a in 0..ACTORS {
            let _ = self
                .stage
                .run(a, Cmd::Clear)
                .expect("an idle actor parked on a clear");
        }
    }

    fn check_woken_after_teardown(&mut self, woken: &[Option<Answer>; ACTORS]) {
        for (b, want) in woken.iter().enumerate() {
            let Some(want) = want else { continue };
            let seq = MAIL[b].lock().seq;
            if !wait_for(|| Stage::finished(b, seq)) {
                mismatch!(self, "teardown should have woken actor {b} with {want:?}");
            }
            let (got, table) = Stage::answer(b);
            if !want.matches(&got) {
                mismatch!(
                    self,
                    "teardown woke actor {b} with {got:?}; the model says {want:?}"
                );
            }
            // A frame cap may or may not survive its region's reclaim; that is not this oracle's.
            let mut table = table;
            for t in table.iter_mut() {
                if matches!(t, Some(c) if matches!(c.object, Object::PageFrame(..))) {
                    *t = None;
                }
            }
            self.check_table(b, &table, &"teardown");
        }
    }
}

fn pick_op(rng: &mut Rng, m: &Model, a: usize) -> Op {
    use abi::{notification as n, page_frame as f, rendezvous as r};
    let held: [u64; SLOTS] = core::array::from_fn(|s| s as u64);
    let held_n = m.tables[a].iter().filter(|c| c.is_some()).count();
    let slot = if held_n > 0 && rng.chance(88) {
        let k = rng.below(held_n as u64) as usize;
        held.iter()
            .copied()
            .filter(|&s| m.tables[a][s as usize].is_some())
            .nth(k)
            .unwrap()
    } else if rng.chance(50) {
        rng.below(SLOTS as u64 + 4)
    } else {
        rng.next()
    };
    if rng.chance(4) {
        return Op::Delete { slot };
    }
    let any_slot = |rng: &mut Rng| {
        if held_n > 0 && rng.chance(80) {
            let k = rng.below(held_n as u64) as usize;
            held.iter()
                .copied()
                .filter(|&s| m.tables[a][s as usize].is_some())
                .nth(k)
                .unwrap()
        } else {
            rng.below(SLOTS as u64 + 2)
        }
    };
    let rights = |rng: &mut Rng| {
        if rng.chance(85) {
            rng.below(16)
        } else {
            rng.next()
        }
    };
    let unknown = rng.chance(4);
    let (method, a) = match m.cap(a, slot).map(|c| c.obj) {
        _ if unknown => (
            if rng.chance(50) {
                NO_METHOD
            } else {
                rng.next()
            },
            [0; 3],
        ),
        Some(Obj::Ep(_)) => match rng.below(13) {
            0..=2 => (r::SEND, [rng.word(), rng.word(), rng.word()]),
            3 | 4 => (r::RECEIVE, [0; 3]),
            5 | 6 => (r::SEND_CAP, [any_slot(rng), rights(rng), rng.word()]),
            7..=9 => (r::RECEIVE_CAP, [0; 3]),
            10 | 11 => (r::CALL, [rng.word(), rng.word(), 0]),
            _ => (
                r::BADGE,
                [
                    if rng.chance(15) {
                        0
                    } else {
                        1 + rng.below(1000)
                    },
                    0,
                    0,
                ],
            ),
        },
        Some(Obj::Reply(_)) => (abi::reply::REPLY, [rng.word(), rng.word(), 0]),
        Some(Obj::Note) => match rng.below(8) {
            0..=2 => (
                n::SIGNAL,
                [
                    if rng.chance(15) {
                        0
                    } else {
                        1 << rng.below(64)
                    },
                    0,
                    0,
                ],
            ),
            3 | 4 => (n::WAIT, [0; 3]),
            5 | 6 => (n::POLL, [0; 3]),
            _ => (n::BIND, [any_slot(rng), 0, 0]),
        },
        Some(Obj::Frame(_)) => (f::REVOKE, [0; 3]),
        // An empty or out-of-range slot: any method, which must not matter.
        None => (rng.below(8), [rng.word(), rng.word(), rng.word()]),
    };
    Op::Invoke { slot, method, a }
}

fn endow(rng: &mut Rng, model: &mut Model, a: usize) -> [Option<MCap>; 8] {
    let mut out = [None; 8];
    let mut k = 0;
    // Half the endowment is whole, so most calls get past the rights check and reach the
    // rendezvous; the other half is any non-empty subset, so every refusal is still reached.
    let rights = |rng: &mut Rng| {
        let r = rng.below(16) as u32;
        if r == 0 || rng.chance(50) {
            Rights::ALL.bits()
        } else {
            r
        }
    };
    for e in 0..EPS {
        for _ in 0..1 + rng.below(2) {
            let badge = if rng.chance(25) { 1 + rng.below(9) } else { 0 };
            out[k] = Some(MCap {
                obj: Obj::Ep(e),
                rights: rights(rng),
                badge,
            });
            k += 1;
        }
    }
    if rng.chance(80) {
        out[k] = Some(MCap {
            obj: Obj::Note,
            rights: rights(rng),
            badge: 0,
        });
        k += 1;
    }
    for f in 0..FRAMES {
        if rng.chance(45) {
            out[k] = Some(MCap {
                obj: Obj::Frame(f),
                rights: rights(rng),
                badge: 0,
            });
            k += 1;
        }
    }
    for c in out.iter().flatten() {
        model.insert(a, *c);
    }
    out
}

fn run_seed(stage: &mut Stage, seed: u64) -> Census {
    let mut rng = Rng(splitmix(seed));
    let region = crate::memory_region::create(8).expect("no region for the fuzz objects");
    let eps = [
        sched::create_rendezvous_from(region).expect("rendezvous 0"),
        sched::create_rendezvous_from(region).expect("rendezvous 1"),
    ];
    let note = sched::create_notification_from(region).expect("notification");
    let frames = core::array::from_fn(|_| {
        crate::memory_region::retype_run(region, 1)
            .expect("frame")
            .0
    });
    let mut run = Run {
        seed,
        step: 0,
        model: Model::new(),
        world: World {
            region,
            eps,
            note,
            frames,
        },
        stage,
    };

    for a in 0..WITNESS {
        let caps = endow(&mut rng, &mut run.model, a);
        for c in caps.iter().flatten() {
            let cap = real_cap(*c, &run.world);
            let (got, _) = run.stage.run(a, Cmd::Grant(cap)).expect("a grant parked");
            assert!(got.r.is_ok(), "seed {seed:#x}: endowing actor {a} failed");
        }
    }

    let mut steps = 0;
    while steps < STEPS {
        let idle: [bool; ACTORS] = core::array::from_fn(|a| run.model.park[a] == Park::Idle);
        if !idle[..WITNESS].iter().any(|&i| i) {
            break; // every endowed actor is parked; only teardown moves them now
        }
        let a = if rng.chance(8) {
            WITNESS
        } else {
            let k = rng.below(idle[..WITNESS].iter().filter(|&&i| i).count() as u64) as usize;
            (0..WITNESS).filter(|&a| idle[a]).nth(k).unwrap()
        };
        let op = pick_op(&mut rng, &run.model, a);
        run.step = steps;
        run.one(a, op);
        steps += 1;
    }
    run.step = steps;
    run.teardown();
    run.model.census
}

/// `NIFE_SYSCALL_FUZZ_SEEDS=<first>:<count>` from the build environment (the weekly sweep, or one
/// seed replayed), or `None` for the committed list.
fn sweep() -> Option<(u64, u64)> {
    let spec = option_env!("NIFE_SYSCALL_FUZZ_SEEDS")?;
    let parse = |s: &str| {
        let s = s.trim();
        match s.strip_prefix("0x") {
            Some(h) => u64::from_str_radix(h, 16).ok(),
            None => s.parse().ok(),
        }
    };
    let (first, count) = spec
        .split_once(':')
        .expect("NIFE_SYSCALL_FUZZ_SEEDS is <first>:<count>");
    Some((
        parse(first).expect("NIFE_SYSCALL_FUZZ_SEEDS: a bad first seed"),
        parse(count).expect("NIFE_SYSCALL_FUZZ_SEEDS: a bad count"),
    ))
}

fn tally(total: &mut Census, c: Census) {
    total.calls += c.calls;
    total.refused += c.refused;
    total.parked += c.parked;
    total.delegated += c.delegated;
    total.replied += c.replied;
    total.revoked += c.revoked;
    total.staged_frame_revoked += c.staged_frame_revoked;
    total.cap_to_plain_receive += c.cap_to_plain_receive;
}

/// **A seeded run of random syscalls agrees with the shadow model on every answer and every
/// table** (milestone 752, provisional). See the module header for the oracle, the witness and the
/// replay.
///
/// Name: provisional (milestone 752's lane), as are the module's and `NIFE_SYSCALL_FUZZ_SEEDS`.
#[test_case]
fn seeded_syscalls_agree_with_the_shadow_model() {
    let cmd_region = crate::memory_region::create(ACTORS as u64 + 1).expect("no doorbell region");
    let doorbells: [RendezvousId; ACTORS] =
        core::array::from_fn(|_| sched::create_rendezvous_from(cmd_region).expect("doorbell"));
    for d in &DONE {
        d.store(0, Ordering::SeqCst);
    }
    let tids: [ThreadId; ACTORS] = core::array::from_fn(|i| {
        let bell = doorbells[i];
        sched::spawn(move || actor(i, bell)).expect("no actor thread")
    });
    let mut stage = Stage {
        tids,
        doorbells,
        seq: 0,
    };

    let hz = crate::arch::timer::frequency();
    let start = crate::arch::timer::now();
    let mut total = Census::default();
    let mut ran = 0u64;
    match sweep() {
        None => {
            for seed in (0..SUITE_SEEDS).chain(CORPUS.iter().copied()) {
                tally(&mut total, run_seed(&mut stage, seed));
                ran += 1;
            }
            crate::println!("    syscall fuzzer: seeds 0..{SUITE_SEEDS} and corpus {CORPUS:?}");
        }
        Some((first, count)) => {
            let cap = start + SWEEP_SECS * hz;
            for seed in first..first.saturating_add(count) {
                if crate::arch::timer::now() > cap {
                    break;
                }
                tally(&mut total, run_seed(&mut stage, seed));
                ran += 1;
            }
            crate::println!(
                "    syscall fuzzer: sweep of seeds {first}..{} ({ran} of {count} asked; a time cap \
                 of {SWEEP_SECS} s stops it early)",
                first + ran
            );
        }
    }
    let ms = (crate::arch::timer::now() - start) * 1000 / hz;
    crate::println!(
        "    syscall fuzzer: {ran} seeds, {ms} ms ({} seeds/s), {total:?}",
        (ran * 1000).checked_div(ms).unwrap_or(0)
    );

    for a in 0..ACTORS {
        let seq = stage.post(a, Cmd::Exit);
        assert!(
            wait_for(|| Stage::finished(a, seq)),
            "actor {a} did not exit"
        );
    }
    assert!(
        wait_for(|| tids.iter().all(|&t| !sched::is_thread_present(t))),
        "an actor thread outlived the run"
    );
    assert!(
        wait_for(|| sched::reclaim_region(cmd_region).is_ok()),
        "the doorbell region could not be reclaimed"
    );
}
