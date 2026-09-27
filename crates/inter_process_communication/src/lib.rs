//! The synchronous-rendezvous state machine (DECISIONS §14, milestone 18; intrusive as of
//! milestone 14 phase A.3).
//!
//! This owns the decision core of `kernel/src/sched.rs`'s IPC: the two wait queues and the
//! pending-signal count, and what a send, a receive, or a signal *does* with them. The kernel
//! wraps it with the bookkeeping the queues cannot express (mailboxes, waking a thread onto a
//! run queue, the one-shot Reply that leaves a caller blocked); the *policy* lives here, proved,
//! and the scheduler calls it rather than hand-rolling the same branch six times.
//!
//! The wait queues are **intrusive** (`crates/intrusive_fifo`): generic over the node type, so in the
//! kernel a queue entry *is* the TCB, threaded through the same link the run queues use. One link
//! means one queue, so "a blocked thread waits on exactly one rendezvous" is a property of there
//! being one field, not a rule anyone keeps. The queues are the kernel's real rendezvous state, not
//! a model kept in sync; what changed at A.3 is only what a queue entry is (a TCB pointer, no
//! longer a `ThreadId` to be looked up) and that queueing can no longer allocate.
//!
//! The load-bearing invariant, unchanged since the original `Rendezvous`: **"at most one wait
//! queue is ever non-empty."** A sender that finds a receiver rendezvouses instead of joining a
//! queue, so a thread only queues when nobody was waiting for it. Every operation is proved to
//! preserve it, now over the real intrusive queues (the `Fifo`'s own FIFO correctness is proved
//! separately, in its crate; here we prove the *decisions* made over it).
//!
//! # Examples
//!
//! The three decisions an rendezvous makes, and the invariant holding across all of them. `T` is the
//! kernel's TCB; here it is a stand-in with the same one link, because one link is the whole reason
//! the invariant is structural rather than remembered.
//!
//! ```
//! use core::ptr::NonNull;
//! use intrusive_fifo::Node;
//! use inter_process_communication::{Rendezvous, Recv, Send};
//!
//! struct ThreadControlBlock {
//!     next: Option<NonNull<ThreadControlBlock>>,
//! }
//!
//! // SAFETY: plain field storage, which is the whole of the `Node` contract.
//! unsafe impl Node for ThreadControlBlock {
//!     fn next(&self) -> Option<NonNull<Self>> {
//!         self.next
//!     }
//!     fn set_next(&mut self, next: Option<NonNull<Self>>) {
//!         self.next = next;
//!     }
//! }
//!
//! // Declared before the rendezvous, so they outlive it.
//! let mut server = ThreadControlBlock { next: None };
//! let mut client = ThreadControlBlock { next: None };
//! let mut ep: Rendezvous<ThreadControlBlock> = Rendezvous::new();
//! assert!(ep.is_idle());
//!
//! // The server calls recv with nobody sending, so it queues. The caller blocks it.
//! // SAFETY: both are live locals declared before `ep`, on no queue, and this is the only accessor.
//! let waiting = unsafe { ep.recv(NonNull::from(&mut server)) };
//! assert_eq!(waiting, Recv::Blocked);
//! assert!(!ep.is_idle());
//! assert!(ep.one_queue_invariant());
//!
//! // Now a client sends. There is a receiver, so it **rendezvouses instead of queueing**, which is
//! // why at most one of the two queues is ever non-empty.
//! // SAFETY: as above.
//! let met = unsafe { ep.send(NonNull::from(&mut client)) };
//! assert_eq!(met, Send::Rendezvous(NonNull::from(&mut server)));
//! assert!(ep.is_idle()); // the receiver left the queue and the sender never joined one
//! assert!(ep.one_queue_invariant());
//! ```
//!
//! A signal is the operation that is deliberately **not** a rendezvous: it never queues the signaller
//! and it is never lost, which is what lets an interrupt handler use one.
//!
//! ```
//! # use core::ptr::NonNull;
//! # use intrusive_fifo::Node;
//! # use inter_process_communication::{Rendezvous, Recv};
//! # struct ThreadControlBlock { next: Option<NonNull<ThreadControlBlock>> }
//! # unsafe impl Node for ThreadControlBlock {
//! #     fn next(&self) -> Option<NonNull<Self>> { self.next }
//! #     fn set_next(&mut self, next: Option<NonNull<Self>>) { self.next = next; }
//! # }
//! let mut driver = ThreadControlBlock { next: None };
//! let mut ep: Rendezvous<ThreadControlBlock> = Rendezvous::new();
//!
//! // Two interrupts arrive with nobody in recv. Neither is dropped; both are counted.
//! assert!(ep.signal().is_none());
//! assert!(ep.signal().is_none());
//!
//! // The driver's next two receives drain them, and it never blocks.
//! // SAFETY: `driver` is a live local declared before `ep`, on no queue.
//! assert_eq!(unsafe { ep.recv(NonNull::from(&mut driver)) }, Recv::Signal);
//! // SAFETY: as above.
//! assert_eq!(unsafe { ep.recv(NonNull::from(&mut driver)) }, Recv::Signal);
//! // The third finds nothing left and queues.
//! // SAFETY: as above.
//! assert_eq!(unsafe { ep.recv(NonNull::from(&mut driver)) }, Recv::Blocked);
//!
//! // And a signal arriving now wakes it, already dequeued.
//! assert_eq!(ep.signal(), Some(NonNull::from(&mut driver)));
//! assert!(ep.is_idle());
//! ```
//!
//! Name: ratified 2026-09-18 (calef, `design/decisions/` §154), **deratifying the 2026-08-01
//! ratification** to do it, and performed 2026-09-19. Refused `ipc`.
//!
//! §154's test is whether the expansion is a phrase people actually say. "inter-process communication" is,
//! so it goes, where `pci` stays because "peripheral component interconnect" is not.
//!
//! The 2026-08-01 block called this one of the standard terms "already right and must not be
//! touched". That was an exemption rather than a test, and §154 records the three-layer
//! contradiction the exemptions left behind.
//!
//! **Performed 2026-09-19**, the last of §154's renames. No public type or fuzz target carried the
//! acronym, so only the crate moved. **IPC the concept did not move with it**: it keeps its name
//! when this crate is deleted, so the lock §118 named and its type, the syscall-path
//! `ipc_send`/`ipc_recv`/`ipc_call`/`ipc_reply` family, `notes/ipc-naming.md` and the word in prose
//! all stay.
//!
//! Census of lowercase `ipc` as a word, outside this file: 206 before, 138 after. Of the
//! survivors, 61 name the concept's own files (`notes/ipc-naming.md`,
//! `notes/ipc-tables-lock-inventory.md`, roadmap and decision slugs), 35 are in `design/decisions/`
//! or `design/naming.md`, which this rename did not edit, 25 are the old name in an account or a
//! measurement, 9 are the concept in code (`bench`'s "ipc server", `board_console`'s `ipc` field),
//! and 5 are `ipc::Endpoint` pointers that were already stale when `Endpoint` became `Rendezvous`
//! (§113) and were deliberately not repointed. The last three are a quoted command and the clause
//! beside it, and `README.md`'s crate list, whose other entries are stale too.

#![cfg_attr(not(test), no_std)]

use core::ptr::NonNull;

use intrusive_fifo::{Fifo, Node};

pub mod notification;

/// One IPC rendezvous: two intrusive wait queues and the pending-signal count.
pub struct Rendezvous<T: Node> {
    /// Senders blocked here, waiting for a receiver.
    senders: Fifo<T>,
    /// Receivers blocked here, waiting for a sender.
    receivers: Fifo<T>,
    /// Async signals that arrived with nobody waiting. Drained by the next receive, never lost.
    pending: u32,
}

/// What a [`send`](Rendezvous::send) decided.
pub enum Send<T> {
    /// A receiver was waiting: rendezvous with this one, and the sender does not join a queue.
    Rendezvous(NonNull<T>),
    /// Nobody was waiting: the sender is now queued on this rendezvous.
    Blocked,
}

/// What a [`recv`](Rendezvous::recv) decided.
pub enum Recv<T> {
    /// A pending async signal was drained; the receiver does not block.
    Signal,
    /// This queued sender was collected; the caller decides whether to wake it.
    FromSender(NonNull<T>),
    /// Nobody was waiting: the receiver is now queued on this rendezvous.
    Blocked,
}

// Manual impls rather than derives: a derive would demand `T: PartialEq`/`T: Debug` even though
// only the *pointer* is stored and compared, and the kernel's `T` (a TCB) is neither.
impl<T> PartialEq for Send<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Send::Rendezvous(a), Send::Rendezvous(b)) => a == b,
            (Send::Blocked, Send::Blocked) => true,
            _ => false,
        }
    }
}
impl<T> Eq for Send<T> {}
impl<T> core::fmt::Debug for Send<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Send::Rendezvous(p) => f.debug_tuple("Rendezvous").field(p).finish(),
            Send::Blocked => f.write_str("Blocked"),
        }
    }
}

impl<T> PartialEq for Recv<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Recv::Signal, Recv::Signal) => true,
            (Recv::FromSender(a), Recv::FromSender(b)) => a == b,
            (Recv::Blocked, Recv::Blocked) => true,
            _ => false,
        }
    }
}
impl<T> Eq for Recv<T> {}
impl<T> core::fmt::Debug for Recv<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Recv::Signal => f.write_str("Signal"),
            Recv::FromSender(p) => f.debug_tuple("FromSender").field(p).finish(),
            Recv::Blocked => f.write_str("Blocked"),
        }
    }
}

impl<T: Node> Rendezvous<T> {
    /// An idle rendezvous: both wait queues empty, no pending signal. `const` so the kernel can
    /// build the rendezvous table at compile time rather than at boot.
    pub const fn new() -> Self {
        Self {
            senders: Fifo::new(),
            receivers: Fifo::new(),
            pending: 0,
        }
    }

    /// **At most one wait queue is ever non-empty.** The load-bearing invariant.
    ///
    /// Name: provisional, flagged 2026-09-24 by the boolean-predicate pass
    /// (design/naming/boolean-predicates-worklist.md). It does not yet follow the Rust predicate
    /// rule calef ratified 2026-09-24; recommended `one_queue_invariant_holds`, because a noun
    /// phrase; the verb form keeps the invariant's name visible.
    pub fn one_queue_invariant(&self) -> bool {
        self.senders.is_empty() || self.receivers.is_empty()
    }

    /// **No thread is blocked on this rendezvous** (both wait queues empty). The pending signal count
    /// does not count: a signal holds no thread.
    pub fn is_idle(&self) -> bool {
        self.senders.is_empty() && self.receivers.is_empty()
    }

    /// Diagnostic: `(queued senders, queued receivers, pending signals)`. For a hang dump, a
    /// nonzero sender count with a zero receiver count on a request rendezvous is a stalled server.
    pub fn debug_counts(&self) -> (usize, usize, u32) {
        (self.senders.len(), self.receivers.len(), self.pending)
    }

    /// **Empty both wait queues, handing every blocked thread back to `f`** (object revocation): the
    /// rendezvous is about to be destroyed, so each waiter is popped off here (which frees its intrusive
    /// link, so `f` may re-queue it onto a run queue) and the caller wakes it with an error. After
    /// this both queues are empty, so [`is_idle`](Self::is_idle) holds and the one-queue invariant
    /// trivially does.
    pub fn drain_waiters(&mut self, mut f: impl FnMut(NonNull<T>)) {
        while let Some(w) = self.senders.pop_front() {
            f(w);
        }
        while let Some(w) = self.receivers.pop_front() {
            f(w);
        }
    }

    /// **Take one specific sender back off the queue**, returning whether it was there.
    ///
    /// The one operation an intrusive `Fifo` deliberately does not offer (arbitrary remove), needed
    /// here for one reason: a **corpse** can be a queued sender. A supervised thread that dies with
    /// nobody in `RECV` parks on its supervision rendezvous's sender queue with the death message in
    /// its mailbox (DECISIONS §26 implementation note 2), and its supervisor may then reap it
    /// (§32's rendezvous reap, or §16's `DESTROY`) *without* having collected the message. Freeing a
    /// TCB that is still linked into a queue leaves a dangling pointer the next `recv` would follow,
    /// so the reap has to unlink it first.
    ///
    /// Expressed as drain-and-repush over `pop_front`/`push_back` rather than as a `Fifo::remove`,
    /// which keeps the "one link, no arbitrary remove" contract intact in the queue itself: the cost
    /// is O(queued senders) on a teardown path, and the queue's own proved invariants are the only
    /// ones in play. FIFO order among the survivors is preserved.
    ///
    /// # Safety
    ///
    /// `victim` is compared by pointer, never dereferenced. Every *other* queued sender is popped
    /// and pushed again, so they must all still satisfy the queue's contract (they do: a queued
    /// sender is blocked or a corpse, and neither is freed while linked).
    pub unsafe fn remove_sender(&mut self, victim: NonNull<T>) -> bool {
        let mut kept: Fifo<T> = Fifo::new();
        let mut found = false;
        while let Some(node) = self.senders.pop_front() {
            if node == victim {
                found = true;
            } else {
                // SAFETY: just popped from this queue, so it is valid and on no queue.
                unsafe { kept.push_back(node) };
            }
        }
        self.senders = kept;
        found
    }

    /// **Take one specific receiver back off the queue**, returning whether it was there. The twin
    /// of [`remove_sender`](Self::remove_sender), and the same drain-and-repush for the same
    /// reason.
    ///
    /// It exists for milestone 133: `MemoryRegion::DESTROY` ends a resident thread that is
    /// permanently `Blocked`, and the commonest such thread is a server parked in `RECV` on a
    /// rendezvous that belongs to somebody else. Its TCB is linked here, so the region's reclaim
    /// has to unlink it before freeing the page the TCB sits on, exactly as a corpse on a
    /// supervision rendezvous's sender queue does.
    ///
    /// **Why both twins rather than one `remove` taking a queue**, which was the tidier shape and
    /// was refused: the caller would then have to name the queue, and naming it means trusting the
    /// victim's recorded [`WaitRole`](crate) to say which one it is. The kernel's `CALL` caller
    /// that met no server is recorded as a `Reply` and *is* on the sender queue, so the role is a
    /// diagnostic rather than the fact. Two pointer-compared removes let the caller ask both
    /// queues and believe neither.
    ///
    /// # Safety
    ///
    /// As for [`remove_sender`](Self::remove_sender): `victim` is compared by pointer and never
    /// dereferenced, and every other queued receiver is popped and pushed again, so they must all
    /// still satisfy the queue's contract.
    pub unsafe fn remove_receiver(&mut self, victim: NonNull<T>) -> bool {
        let mut kept: Fifo<T> = Fifo::new();
        let mut found = false;
        while let Some(node) = self.receivers.pop_front() {
            if node == victim {
                found = true;
            } else {
                // SAFETY: just popped from this queue, so it is valid and on no queue.
                unsafe { kept.push_back(node) };
            }
        }
        self.receivers = kept;
        found
    }

    /// A sender `me` arrives. Rendezvous with a waiting receiver if there is one, otherwise `me`
    /// joins the sender queue (and the caller should block it).
    ///
    /// # Safety
    ///
    /// `me` must satisfy the intrusive contract: valid, on no queue, and it must stay valid for
    /// as long as it may be queued here. (The kernel's discipline: `me` is the running thread,
    /// and a thread queued here is `Blocked`, which the reaper never touches.)
    pub unsafe fn send(&mut self, me: NonNull<T>) -> Send<T> {
        if let Some(receiver) = self.receivers.pop_front() {
            Send::Rendezvous(receiver)
        } else {
            // SAFETY: the caller's contract is exactly the queue's.
            unsafe { self.senders.push_back(me) };
            Send::Blocked
        }
    }

    /// A receiver `me` arrives. Drain a pending signal first (never lose one behind a later
    /// sender), then collect a queued sender, otherwise `me` joins the receiver queue (and the
    /// caller should block it).
    ///
    /// # Safety
    ///
    /// As for [`send`](Self::send).
    pub unsafe fn recv(&mut self, me: NonNull<T>) -> Recv<T> {
        if self.pending > 0 {
            self.pending -= 1;
            Recv::Signal
        } else if let Some(sender) = self.senders.pop_front() {
            Recv::FromSender(sender)
        } else {
            // SAFETY: the caller's contract is exactly the queue's.
            unsafe { self.receivers.push_back(me) };
            Recv::Blocked
        }
    }

    /// An async signal arrives. Wake a waiting receiver (returned, already dequeued), or count it
    /// for the next receive. **Not a rendezvous:** it never joins the sender queue and is never
    /// lost. Safe: signalling queues nothing.
    pub fn signal(&mut self) -> Option<NonNull<T>> {
        if let Some(receiver) = self.receivers.pop_front() {
            Some(receiver)
        } else {
            self.pending = self.pending.saturating_add(1);
            None
        }
    }
}

impl<T: Node> Default for Rendezvous<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Machine-checked proofs of the rendezvous state machine (DECISIONS §14, milestone 18; restated
/// over the intrusive queues at milestone 14 phase A.3, so the rewire did not demote proved code
/// back to argued code).
///
/// Every operation is proved to preserve the one-queue invariant, and each decision is proved to
/// match the rule. These are inductive-step proofs: assume a valid state, apply one operation,
/// check. A non-empty queue is modeled with a single waiter, because the decision and the
/// invariant depend only on whether a queue is *empty*, never on its length: an operation pops
/// (only shrinking) and pushes only to a queue that was empty, so the emptiness pattern
/// transitions identically for one waiter or many. FIFO order within a queue is the `intrusive_fifo`
/// crate's own proof; these harnesses prove the decisions made over it.
///
/// # The obligations every `unsafe` call in here discharges
///
/// Stated once, rather than re-derived at each of the eleven sites; each site's own comment adds
/// only what is particular to it. (The `#[cfg(test)]` module below does the same thing for the same
/// reason.)
///
/// **Every node outlives the rendezvous.** Each harness declares its `N`s in one `let` before it
/// declares `e`, and Rust drops locals in reverse declaration order, so the `Rendezvous` is destroyed
/// first. A node still parked on a queue when the harness returns was therefore valid for the whole
/// of its time there, which is the "stays valid for as long as it may be queued" half of
/// `send`/`recv`'s contract and of `seed`'s.
///
/// **Every node is on no queue when it is passed.** Each `N::new()` starts with a null link, `e`
/// starts empty, and no harness hands the same node to two calls. That is why the harnesses carry a
/// separate `me` (and, in one case, a `me2`) rather than reusing `s` or `r`: it makes this
/// obligation a fact about how many locals there are, not an argument about what the previous
/// operation decided.
///
/// Both are properties the *harness* has, not properties Kani checks. Kani would catch a dangling
/// dereference if one of these were false and a proof reached it, but nothing here proves the
/// contract is kept; that is what the comments are for.
#[cfg(kani)]
mod verification {
    use super::*;

    /// A minimal node: a link and nothing else, the way the proofs like it.
    struct N {
        next: Option<NonNull<N>>,
    }

    impl N {
        fn new() -> Self {
            N { next: None }
        }
    }

    // SAFETY: `next` and `set_next` read and write the same `next` field and nothing else, which is
    // the whole of the `Node` contract.
    unsafe impl Node for N {
        fn next(&self) -> Option<NonNull<Self>> {
            self.next
        }
        fn set_next(&mut self, next: Option<NonNull<Self>>) {
            self.next = next;
        }
    }

    /// Put `e` into an arbitrary valid state: at most one queue non-empty (modeled as one
    /// waiter), and a symbolic pending count. The waiter nodes are the caller's locals, so they
    /// outlive the rendezvous.
    ///
    /// # Safety
    /// `sender` and `receiver` must be valid, distinct, unqueued nodes outliving `e`.
    unsafe fn seed(e: &mut Rendezvous<N>, sender: NonNull<N>, receiver: NonNull<N>) {
        e.pending = kani::any();
        match kani::any::<u8>() {
            // SAFETY: `sender` is valid, unqueued and outlives `e`, by this function's own
            // contract; the arms are exclusive, so it is pushed at most once.
            0 => unsafe { e.senders.push_back(sender) },
            // SAFETY: the same, for `receiver`, which the contract requires to be a *distinct*
            // node from `sender` and so separately unqueued.
            1 => unsafe { e.receivers.push_back(receiver) },
            _ => {} // both empty
        }
    }

    /// Falsification: replayable `crates/inter_process_communication/falsifications/verification.send_preserves_the_invariant.patch`
    #[kani::proof]
    fn send_preserves_the_invariant() {
        let (mut s, mut r, mut me) = (N::new(), N::new(), N::new());
        let mut e: Rendezvous<N> = Rendezvous::new();
        // SAFETY: `s`, `r` and `me` are three distinct fresh nodes declared before `e`, so each is
        // valid, on no queue, and outlives the rendezvous. That is `seed`'s contract and `send`'s
        // both; `send` gets `me`, which `seed` never touches.
        unsafe {
            seed(&mut e, NonNull::from(&mut s), NonNull::from(&mut r));
            e.send(NonNull::from(&mut me));
        }
        assert!(e.one_queue_invariant());
    }

    /// Falsification: replayable `crates/inter_process_communication/falsifications/verification.recv_preserves_the_invariant.patch`
    #[kani::proof]
    fn recv_preserves_the_invariant() {
        let (mut s, mut r, mut me) = (N::new(), N::new(), N::new());
        let mut e: Rendezvous<N> = Rendezvous::new();
        // SAFETY: as in `send_preserves_the_invariant` above; `recv`'s contract on `me` is `send`'s.
        unsafe {
            seed(&mut e, NonNull::from(&mut s), NonNull::from(&mut r));
            e.recv(NonNull::from(&mut me));
        }
        assert!(e.one_queue_invariant());
    }

    /// Falsification: unfalsified. No minimal defect in `signal` can turn this harness red, and
    /// that is a fact about the operation rather than a gap in the effort. The invariant is "at
    /// most one queue is non-empty", and `signal` takes no node: its two branches pop a receiver
    /// (which can only empty a queue) and increment a counter. Nothing it can be mistakenly
    /// written to do enqueues anything, so every mutation of it either leaves the invariant true
    /// or is caught by a neighbouring harness instead. The harness earns its place as a guard
    /// against `signal` growing an enqueue path later, not as evidence about the code today.
    /// Recording a patch here would mean patching a sibling operation and filing the red under
    /// this name, which is the manufactured evidence DECISIONS §134 (a harness carries a
    /// machine-replayable falsification record, or it is not evidence) exists to refuse.
    #[kani::proof]
    fn signal_preserves_the_invariant() {
        let (mut s, mut r) = (N::new(), N::new());
        let mut e: Rendezvous<N> = Rendezvous::new();
        // SAFETY: `s` and `r` are distinct fresh nodes declared before `e`, so they are valid,
        // unqueued and outlive it. `signal` is safe and takes no node, so `seed` is the only
        // obligation here.
        unsafe { seed(&mut e, NonNull::from(&mut s), NonNull::from(&mut r)) };
        e.signal();
        assert!(e.one_queue_invariant());
    }

    /// **A send rendezvouses exactly when a receiver was waiting**, and with exactly *that*
    /// receiver, else blocks. So a message is never dropped, a sender never blocks past a ready
    /// receiver, and the rendezvous partner is the queued thread and no other.
    /// Falsification: replayable `crates/inter_process_communication/falsifications/verification.send_rendezvous_iff_a_receiver_waited.patch`
    #[kani::proof]
    fn send_rendezvous_iff_a_receiver_waited() {
        let (mut s, mut r, mut me) = (N::new(), N::new(), N::new());
        let receiver_ptr = NonNull::from(&mut r);
        let mut e: Rendezvous<N> = Rendezvous::new();
        // SAFETY: `s` and `r` are distinct fresh nodes declared before `e`, so they are valid,
        // unqueued and outlive it. `receiver_ptr` is `&mut r` taken once and kept, which is the
        // same pointer `seed` would have been given inline; no second pointer to `r` exists.
        unsafe { seed(&mut e, NonNull::from(&mut s), receiver_ptr) };

        let had_receiver = !e.receivers.is_empty();
        // SAFETY: `me` is a third fresh node declared before `e`, so it is valid, on no queue
        // (`seed` was given `s` and `r`, never `me`), and outlives the rendezvous.
        match unsafe { e.send(NonNull::from(&mut me)) } {
            Send::Rendezvous(got) => {
                assert!(had_receiver);
                assert_eq!(
                    got, receiver_ptr,
                    "rendezvoused with a thread nobody queued"
                );
            }
            Send::Blocked => assert!(!had_receiver),
        }
    }

    /// **Taking a waiter back off a queue preserves the one-queue invariant**, for either queue and
    /// whether or not the victim was ever there.
    ///
    /// Milestone 133 needed [`remove_receiver`](Rendezvous::remove_receiver) and asks both removes
    /// on every victim, because the kernel's recorded wait role and the queue that actually holds
    /// the thread genuinely disagree for a `CALL` caller that met no server. So the interesting
    /// case is not the hit, it is the **miss**: a drain-and-repush over a queue the victim is not
    /// on rebuilds that queue from nothing and must put it back exactly as it was. A version that
    /// dropped the survivors, or left both queues non-empty, would break the invariant every other
    /// proof in this module rests on.
    ///
    /// **This one needs a loop bound, and the two facts behind that are worth the paragraph**,
    /// because every other harness in this module is unbounded and finishes in a tenth of a
    /// second. The first shape of it ran both removes twice each and CBMC was still working after
    /// **seventeen minutes**; with `unwind(3)` and a single symbolic remove it takes 0.2 seconds.
    ///
    /// The bound is sound rather than a shortcut: Kani checks the unwinding assertion, so a loop
    /// that needed a fourth iteration would **fail** this proof rather than pass it quietly, and
    /// `seed` queues at most one waiter, so two is the most a drain-and-repush can take. What the
    /// bound costs is exactly what `seed` already costs everywhere in this module: the proof is
    /// over a rendezvous holding at most one waiter, and a queue of many is argued rather than
    /// proved.
    ///
    /// **One remove per run, chosen symbolically, rather than the four the kernel makes**, and
    /// nothing is lost by that: the removes share no state between calls, each reading one queue
    /// and writing it back, so proving one over an arbitrary seeded rendezvous covers a sequence.
    /// Falsification: replayable `crates/inter_process_communication/falsifications/verification.removing_a_waiter_preserves_the_invariant.patch`
    #[kani::proof]
    #[kani::unwind(3)]
    fn removing_a_waiter_preserves_the_invariant() {
        let (mut s, mut r, mut stranger) = (N::new(), N::new(), N::new());
        let (sender_ptr, receiver_ptr) = (NonNull::from(&mut s), NonNull::from(&mut r));
        let mut e: Rendezvous<N> = Rendezvous::new();
        // SAFETY: `s` and `r` are distinct fresh nodes declared before `e`, so they are valid,
        // unqueued and outlive it, which is `seed`'s contract.
        unsafe { seed(&mut e, sender_ptr, receiver_ptr) };

        // A symbolic victim: one of the two nodes `seed` may have queued, or a third that is on no
        // queue at all, which is the miss the kernel takes on every call.
        let victim = match kani::any::<u8>() {
            0 => sender_ptr,
            1 => receiver_ptr,
            _ => NonNull::from(&mut stranger),
        };
        let found = if kani::any() {
            // SAFETY: `victim` is compared by pointer and never dereferenced, and the only other
            // node either queue can hold is one of the same three live locals, each still on at
            // most one queue.
            unsafe { e.remove_sender(victim) }
        } else {
            // SAFETY: as above; the twin removes over the other queue with the same contract.
            unsafe { e.remove_receiver(victim) }
        };
        assert!(e.one_queue_invariant());
        // A remove that reports a hit really emptied the queue it was given: `seed` queues at most
        // one waiter, so a hit leaves both queues empty and the rendezvous idle.
        assert!(!found || e.is_idle());
    }

    /// **A pending signal is taken before a queued sender.** A receive drains a counted signal
    /// first, so an async signal delivered with nobody waiting is never lost behind a later
    /// synchronous sender.
    /// Falsification: replayable `crates/inter_process_communication/falsifications/verification.recv_drains_a_pending_signal_first.patch`
    #[kani::proof]
    fn recv_drains_a_pending_signal_first() {
        let (mut s, mut r, mut me) = (N::new(), N::new(), N::new());
        let mut e: Rendezvous<N> = Rendezvous::new();
        // SAFETY: `s` and `r` are distinct fresh nodes declared before `e`: valid, unqueued,
        // outliving it.
        unsafe { seed(&mut e, NonNull::from(&mut s), NonNull::from(&mut r)) };
        if e.pending > 0 {
            // SAFETY: `me` is a third fresh node declared before `e`, never given to `seed`, so it
            // is valid, on no queue, and outlives the rendezvous. This receive drains a signal rather
            // than queueing `me`, but that is what the harness asserts, not what makes the call
            // sound: the contract is met either way.
            assert_eq!(unsafe { e.recv(NonNull::from(&mut me)) }, Recv::Signal);
        }
    }

    /// **A collected sender is forgotten by the rendezvous.** The rendezvous half of the one-shot
    /// Reply guarantee (DECISIONS §12): a `CALL`er queues as a sender and blocks; when a server's
    /// receive collects it, the pop is destructive, so afterwards the rendezvous holds no name for
    /// the caller in either queue and no later receive can produce it again. From that moment the
    /// kernel-minted Reply capability is the *only* name for the blocked caller anywhere, and the
    /// capability side (consume-on-use, proved in `crates/capability`) makes that name single-use.
    ///
    /// One waiter covers the general case here as everywhere in this module, plus one fact the
    /// queue cannot see: a blocked thread cannot run, so it cannot enqueue itself a second time.
    /// Stated through emptiness (the decision core's own vocabulary; a membership scan would hand
    /// the solver an unbounded loop for no added meaning).
    /// Falsification: replayable `crates/inter_process_communication/falsifications/verification.a_collected_sender_is_forgotten.patch`
    #[kani::proof]
    fn a_collected_sender_is_forgotten() {
        let (mut s, mut r, mut me, mut me2) = (N::new(), N::new(), N::new(), N::new());
        let mut e: Rendezvous<N> = Rendezvous::new();
        // SAFETY: `s` and `r` are distinct fresh nodes declared before `e`: valid, unqueued,
        // outliving it.
        unsafe { seed(&mut e, NonNull::from(&mut s), NonNull::from(&mut r)) };
        if matches!(
            unsafe {
                // SAFETY: `me` is a fresh node declared before `e` and never given to `seed`, so it
                // is valid, on no queue, and outlives the rendezvous.
                e.recv(NonNull::from(&mut me))
            },
            Recv::FromSender(_)
        ) {
            assert!(e.senders.is_empty() && e.receivers.is_empty());
            assert!(!matches!(
                unsafe {
                    // SAFETY: `me2` is a fourth fresh node, declared before `e` and passed to
                    // nothing else. It exists so this second receive does not reuse `me`: `me` was
                    // not queued by the receive above (it returned `FromSender`), but a separate
                    // node makes this site's obligation independent of that reasoning.
                    e.recv(NonNull::from(&mut me2))
                },
                Recv::FromSender(_)
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    //! Every `unsafe` call below satisfies the same two obligations, stated once here rather than
    //! re-derived at each of the twenty-odd sites.
    //!
    //! **The node outlives the rendezvous.** Each test declares its nodes before its `Rendezvous`, and
    //! Rust drops locals in reverse declaration order, so `e` is destroyed first. A node parked on a
    //! queue when the test ends is therefore still valid when the rendezvous goes away, which is the
    //! "stays valid for as long as it may be queued" half of `send`/`recv`'s contract.
    //!
    //! **The node is on no queue when it is passed.** This is the half that is NOT free, because
    //! several tests reuse one receiver node: it holds only because a `recv` that returns `Signal`
    //! or `FromSender` never queues its argument. Where a site depends on that, its own comment
    //! says so.

    use super::*;

    struct N {
        next: Option<NonNull<N>>,
    }

    // SAFETY: `next` and `set_next` read and write the same `next` field and nothing else, which is the whole of the `Node` contract.
    unsafe impl Node for N {
        fn next(&self) -> Option<NonNull<Self>> {
            self.next
        }
        fn set_next(&mut self, next: Option<NonNull<Self>>) {
            self.next = next;
        }
    }

    fn node() -> Box<N> {
        Box::new(N { next: None })
    }

    /// The rendezvous, both orderings: whoever arrives first waits, the second completes the pair
    /// and gets the first: the very node, by identity, not a name to look up.
    #[test]
    fn sender_first_then_receiver_rendezvous() {
        let (mut s, mut r) = (node(), node());
        let sp = NonNull::from(&mut *s);
        let mut e: Rendezvous<N> = Rendezvous::new();

        // SAFETY: `sp` is a live node, on no queue (see the module note).
        assert_eq!(unsafe { e.send(sp) }, Send::Blocked); // nobody waiting: park the sender
        assert_eq!(
            // SAFETY: as above.
            unsafe { e.recv(NonNull::from(&mut *r)) },
            Recv::FromSender(sp)
        ); // receiver collects it
        assert!(e.one_queue_invariant());
    }

    #[test]
    fn receiver_first_then_sender_rendezvous() {
        let (mut s, mut r) = (node(), node());
        let rp = NonNull::from(&mut *r);
        let mut e: Rendezvous<N> = Rendezvous::new();

        // SAFETY: `rp` is a live node, on no queue (see the module note).
        assert_eq!(unsafe { e.recv(rp) }, Recv::Blocked);
        assert_eq!(
            // SAFETY: as above.
            unsafe { e.send(NonNull::from(&mut *s)) },
            Send::Rendezvous(rp)
        ); // sender meets the waiter
    }

    /// Two senders queue in FIFO order; two receivers drain them in the same order.
    #[test]
    fn senders_queue_fifo() {
        let (mut a, mut b, mut r) = (node(), node(), node());
        let (ap, bp) = (NonNull::from(&mut *a), NonNull::from(&mut *b));
        let mut e: Rendezvous<N> = Rendezvous::new();

        // SAFETY: `ap` and `bp` are live nodes, each on no queue (see the module note).
        assert_eq!(unsafe { e.send(ap) }, Send::Blocked);
        // SAFETY: as above.
        assert_eq!(unsafe { e.send(bp) }, Send::Blocked);
        assert_eq!(
            // SAFETY: as above.
            unsafe { e.recv(NonNull::from(&mut *r)) },
            Recv::FromSender(ap)
        );
        // SAFETY: as above; the previous `recv` returned `FromSender`, so `r` was never queued and is still on no queue.
        assert_eq!(
            // SAFETY: as above.
            unsafe { e.recv(NonNull::from(&mut *r)) },
            Recv::FromSender(bp)
        );
    }

    /// A signal with nobody waiting is counted; the next receives drain it, then block.
    #[test]
    fn a_signal_to_an_empty_rendezvous_is_counted_then_drained() {
        let mut r = node();
        let mut e: Rendezvous<N> = Rendezvous::new();

        assert_eq!(e.signal(), None); // counted
        assert_eq!(e.signal(), None);
        // SAFETY: `r` is a live node, on no queue (see the module note). The first two calls drain counted signals and never queue it; only the third parks it.
        assert_eq!(unsafe { e.recv(NonNull::from(&mut *r)) }, Recv::Signal);
        // SAFETY: as above.
        assert_eq!(unsafe { e.recv(NonNull::from(&mut *r)) }, Recv::Signal);
        // SAFETY: as above.
        assert_eq!(unsafe { e.recv(NonNull::from(&mut *r)) }, Recv::Blocked);
    }

    /// The rendezvous-destroy contract (object revocation): `drain_waiters` hands back every parked
    /// thread exactly once, in queue order, and leaves the rendezvous idle. The kernel's `revoke`
    /// wakes each one with an error; if a waiter were skipped it would sleep forever on a dead
    /// rendezvous, and if one were handed back twice it would be double-queued on a run queue.
    #[test]
    fn drain_hands_back_every_waiter_and_leaves_the_rendezvous_idle() {
        let (mut a, mut b, mut r) = (node(), node(), node());
        let (ap, bp) = (NonNull::from(&mut *a), NonNull::from(&mut *b));
        // Via `default()`: the kernel retypes rendezvous pages through it, not through `new()`.
        let mut e: Rendezvous<N> = Rendezvous::default();

        // SAFETY: `ap` and `bp` are live nodes, each on no queue (see the module note).
        assert_eq!(unsafe { e.send(ap) }, Send::Blocked);
        // SAFETY: as above.
        assert_eq!(unsafe { e.send(bp) }, Send::Blocked);
        assert!(!e.is_idle(), "parked senders hold the rendezvous live");

        let mut drained = Vec::new();
        e.drain_waiters(|w| drained.push(w));
        assert_eq!(drained, [ap, bp]);
        assert!(e.is_idle());

        // The other queue drains through the same path: a receiver can be parked too.
        let rp = NonNull::from(&mut *r);
        // SAFETY: `rp` is a live node, on no queue (see the module note); `drain_waiters` emptied the queues above.
        assert_eq!(unsafe { e.recv(rp) }, Recv::Blocked);
        drained.clear();
        e.drain_waiters(|w| drained.push(w));
        assert_eq!(drained, [rp]);
        assert!(e.is_idle());
    }

    /// Pending signals do not hold an rendezvous live: `is_idle` counts blocked threads, not
    /// counters. An rendezvous whose only state is undelivered signals is safe to destroy (a
    /// signal holds no thread, so nobody is left sleeping), and revocation relies on that.
    #[test]
    fn pending_signals_do_not_make_an_rendezvous_busy() {
        let mut e: Rendezvous<N> = Rendezvous::new();
        assert_eq!(e.signal(), None);
        assert_eq!(e.signal(), None);
        assert!(e.is_idle());
    }

    /// **A queued sender can be taken back out of the middle**, which is what reaping a corpse
    /// needs (DECISIONS §32, and §16's `DESTROY` before it): a supervised thread that died with
    /// nobody receiving is parked here with its death message, and freeing it while it is still
    /// linked would leave the next `recv` following a dangling pointer. The survivors keep FIFO
    /// order, the length drops by exactly one, and removing something that is not queued reports
    /// `false` and changes nothing.
    #[test]
    fn a_queued_sender_can_be_removed_from_the_middle() {
        let (mut a, mut b, mut c, mut r) = (node(), node(), node(), node());
        let (ap, bp, cp) = (
            NonNull::from(&mut *a),
            NonNull::from(&mut *b),
            NonNull::from(&mut *c),
        );
        let mut e: Rendezvous<N> = Rendezvous::new();

        for p in [ap, bp, cp] {
            // SAFETY: `ap`, `bp` and `cp` are live nodes, each on no queue (see the module note).
            assert_eq!(unsafe { e.send(p) }, Send::Blocked);
        }
        // SAFETY: `bp` is compared by pointer and never dereferenced; the senders that get re-queued are the same live locals.
        assert!(unsafe { e.remove_sender(bp) }, "b was queued");
        assert_eq!(e.debug_counts().0, 2, "exactly one sender left the queue");
        assert_eq!(
            // SAFETY: as above.
            unsafe { e.recv(NonNull::from(&mut *r)) },
            Recv::FromSender(ap)
        );
        assert_eq!(
            // SAFETY: as above.
            unsafe { e.recv(NonNull::from(&mut *r)) },
            Recv::FromSender(cp)
        );
        // SAFETY: as above.
        assert_eq!(unsafe { e.recv(NonNull::from(&mut *r)) }, Recv::Blocked);

        // Not queued (already collected, the ordinary case): a no-op that says so.
        let mut e2: Rendezvous<N> = Rendezvous::new();
        // SAFETY: `ap` is not queued on `e2` at all, and `remove_sender` only compares pointers, so nothing is dereferenced.
        assert!(!unsafe { e2.remove_sender(ap) });
        assert!(e2.is_idle());
    }

    /// Removing the *only* queued sender leaves the rendezvous idle rather than a queue with a stale
    /// tail: the classic drained-to-empty bug, which matters here because the single-corpse case is
    /// the common one.
    #[test]
    fn removing_the_only_sender_leaves_the_rendezvous_idle() {
        let (mut a, mut r) = (node(), node());
        let ap = NonNull::from(&mut *a);
        let mut e: Rendezvous<N> = Rendezvous::new();

        // SAFETY: `ap` is a live node, on no queue (see the module note).
        assert_eq!(unsafe { e.send(ap) }, Send::Blocked);
        // SAFETY: `ap` is compared by pointer, never dereferenced.
        assert!(unsafe { e.remove_sender(ap) });
        assert!(e.is_idle(), "the rendezvous still holds a sender");
        // SAFETY: as above.
        assert_eq!(unsafe { e.recv(NonNull::from(&mut *r)) }, Recv::Blocked);
        // And it can be used again afterwards: push, pop, no ghost.
        assert!(e.one_queue_invariant());
    }

    /// **A queued receiver can be taken back out of the middle**, which is what milestone 133's
    /// completed reclaim needs: a server parked in `RECV` on somebody else's rendezvous is linked
    /// here, and `MemoryRegion::DESTROY` on the region holding its TCB frees the page that link
    /// points into. The survivors keep FIFO order, the count drops by exactly one, and removing
    /// something that is not queued reports `false` and changes nothing.
    #[test]
    fn a_queued_receiver_can_be_removed_from_the_middle() {
        let (mut a, mut b, mut c, mut sdr) = (node(), node(), node(), node());
        let (ap, bp, cp) = (
            NonNull::from(&mut *a),
            NonNull::from(&mut *b),
            NonNull::from(&mut *c),
        );
        let mut e: Rendezvous<N> = Rendezvous::new();

        for p in [ap, bp, cp] {
            // SAFETY: `ap`, `bp` and `cp` are live nodes, each on no queue (see the module note).
            assert_eq!(unsafe { e.recv(p) }, Recv::Blocked);
        }
        // SAFETY: `bp` is compared by pointer and never dereferenced; the receivers that get re-queued are the same live locals.
        assert!(unsafe { e.remove_receiver(bp) }, "b was queued");
        assert_eq!(e.debug_counts().1, 2, "exactly one receiver left the queue");
        assert_eq!(
            // SAFETY: as above.
            unsafe { e.send(NonNull::from(&mut *sdr)) },
            Send::Rendezvous(ap)
        );
        assert_eq!(
            // SAFETY: as above.
            unsafe { e.send(NonNull::from(&mut *sdr)) },
            Send::Rendezvous(cp)
        );

        // Not queued: a no-op that says so, and it does not reach into the *sender* queue either.
        let mut e2: Rendezvous<N> = Rendezvous::new();
        // SAFETY: `ap` is a live node on no queue of `e2`; nothing is dereferenced.
        assert_eq!(unsafe { e2.send(ap) }, Send::Blocked);
        // SAFETY: `ap` is compared by pointer, never dereferenced.
        let found = unsafe { e2.remove_receiver(ap) };
        assert!(!found, "a queued sender is not a receiver");
        assert_eq!(e2.debug_counts().0, 1, "the sender queue was left alone");
    }

    /// Removing the *only* queued receiver leaves the rendezvous idle rather than a queue with a
    /// stale tail, the same drained-to-empty check `remove_sender` carries. A lone blocked server
    /// is the common shape of the thread milestone 133 ends.
    #[test]
    fn removing_the_only_receiver_leaves_the_rendezvous_idle() {
        let (mut a, mut sdr) = (node(), node());
        let ap = NonNull::from(&mut *a);
        let mut e: Rendezvous<N> = Rendezvous::new();

        // SAFETY: `ap` is a live node, on no queue (see the module note).
        assert_eq!(unsafe { e.recv(ap) }, Recv::Blocked);
        // SAFETY: `ap` is compared by pointer, never dereferenced.
        assert!(unsafe { e.remove_receiver(ap) });
        assert!(e.is_idle(), "the rendezvous still holds a receiver");
        // SAFETY: as above.
        assert_eq!(unsafe { e.send(NonNull::from(&mut *sdr)) }, Send::Blocked);
        assert!(e.one_queue_invariant());
    }

    /// A signal with a receiver waiting hands it back directly and counts nothing.
    #[test]
    fn a_signal_wakes_a_waiting_receiver() {
        let mut r = node();
        let rp = NonNull::from(&mut *r);
        let mut e: Rendezvous<N> = Rendezvous::new();

        // SAFETY: `rp` is a live node, on no queue (see the module note).
        assert_eq!(unsafe { e.recv(rp) }, Recv::Blocked);
        assert_eq!(e.signal(), Some(rp)); // the waiter, dequeued
        assert!(e.one_queue_invariant());
    }

    /// The manual `PartialEq` and `Debug` impls answer for themselves. Every use above compares
    /// equal values, so an eq stuck at `true` passed (milestone 85); different variants must
    /// disagree, and the rendering is the string a hang dump prints.
    #[test]
    fn verdict_variants_are_distinct_and_print_their_names() {
        let mut s = node();
        let sp = NonNull::from(&mut *s);
        assert_ne!(Send::<N>::Blocked, Send::Rendezvous(sp));
        assert_ne!(Recv::<N>::Signal, Recv::Blocked);
        assert_ne!(Recv::<N>::FromSender(sp), Recv::Signal);
        assert_eq!(format!("{:?}", Send::<N>::Blocked), "Blocked");
        assert_eq!(format!("{:?}", Recv::<N>::Signal), "Signal");
        assert!(format!("{:?}", Recv::<N>::FromSender(sp)).starts_with("FromSender"));
    }
}
