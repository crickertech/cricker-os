//! An intrusive FIFO queue: **the link lives inside the node.**
//!
//! Milestone 14 phase A.2 (design/kernel-objects-from-untyped.md, decision D1). The per-CPU run
//! queues and migration inboxes used to be `VecDeque<ThreadId>`: every entry was heap-backed, pushing
//! could allocate (which is why the scheduler pre-reserved capacity, a standing apology to the
//! IRQ path), and every pop handed back a number that still had to be looked up.
//!
//! An intrusive queue stores nothing. The "next" pointer lives inside the thing being queued,
//! and the queue itself is two pointers, head and tail. Push and pop are a couple of pointer
//! writes, they cannot allocate and cannot fail, and what a pop hands back is the object itself.
//! This is what every kernel we learn from does (Linux `list_head`, seL4's TCB queues), for
//! exactly these reasons.
//!
//! # The price, stated plainly
//!
//! **One link means one queue.** A node can be on at most one `Fifo` at a time, because there is
//! only one `next` inside it. For a scheduler this is not a restriction, it is the *invariant*:
//! a thread is ready on one core's queue, or parked in one inbox, or blocked on one endpoint, or
//! running, and never two of those at once. The queue makes the state machine physical.
//!
//! **And the compiler checks half of it.** A `VecDeque` owns its entries; this queue borrows its
//! nodes with no lifetime the borrow checker can see. What it can see is an [`Unqueued`] token:
//! a non-`Clone` proof that one node is on no queue. A push consumes one and a pop hands one back,
//! so "on at most one queue" is the compiler's fact rather than the caller's (milestone 139 (drive
//! the unsafe count down), round 10; calef's ruling C on #1842). The only `unsafe` left to a caller
//! is minting a token, once per node, which is where rule 2 (the node outlives its time on a queue)
//! is still argued by hand.
//!
//! # The caller's contract
//!
//! 1. A node is pushed onto at most one queue, and not pushed again until popped. **Enforced**:
//!    a push takes the node's [`Unqueued`] token by value, and only a pop gives it back.
//! 2. A node outlives its time on the queue (the kernel: a queued thread is `Ready`, and only
//!    `Finished` threads are ever freed). Stated once, at [`Unqueued::new`].
//! 3. All access to a queue and its nodes' links is serialized by the caller (the kernel: a run
//!    queue is single-core with interrupts masked; an inbox is behind its mutex). Enforced for the
//!    queue by `&mut self`; for the links, also stated at [`Unqueued::new`].
//!
//! # Examples
//!
//! A run queue, with the three contract rules visible in the code rather than only in the list
//! above. Note where the nodes are declared: **before** the queue, so they outlive it, because
//! locals drop in reverse and rule 2 is otherwise nothing but a promise.
//!
//! ```
//! use core::ptr::NonNull;
//! use intrusive_fifo::{Fifo, Node, Unqueued};
//!
//! struct Thread {
//!     tid: u32,
//!     next: Option<NonNull<Thread>>,
//! }
//!
//! // SAFETY: `next` and `set_next` are plain field storage and touch nothing else, which is the
//! // whole of the `Node` contract. A clever implementation here is a corrupted queue.
//! unsafe impl Node for Thread {
//!     fn next(&self) -> Option<NonNull<Self>> {
//!         self.next
//!     }
//!     fn set_next(&mut self, next: Option<NonNull<Self>>) {
//!         self.next = next;
//!     }
//! }
//!
//! // Rule 2, as drop order: these outlive `q`.
//! let mut a = Thread { tid: 1, next: None };
//! let mut b = Thread { tid: 2, next: None };
//! let mut q: Fifo<Thread> = Fifo::new();
//!
//! // The one unsafe step: minting each node's token, once. Both are live locals declared before
//! // `q`, neither is on a queue and neither has a token yet, and this doctest is the only accessor.
//! let (ta, tb) = unsafe { (Unqueued::new(NonNull::from(&mut a)), Unqueued::new(NonNull::from(&mut b))) };
//!
//! // Pushing is safe: the token is the proof that the node is on no queue, and the push takes it.
//! q.push_back(ta);
//! q.push_back(tb);
//! assert_eq!(q.len(), 2);
//! assert!(!q.is_empty());
//!
//! // Round robin: threads leave in the order they arrived, and a pop hands back the token, which is
//! // the object itself rather than a number somebody has to look up.
//! let first = q.pop_front().unwrap();
//! // SAFETY: the token's node is one of the live locals above.
//! assert_eq!(unsafe { (*first.as_ptr()).tid }, 1);
//!
//! // A drained-then-refilled queue takes nodes correctly, which is where the classic tail-pointer
//! // bug lives; and the popped token is what makes pushing `a` again legal.
//! q.push_back(first);
//! let order: Vec<u32> = core::iter::from_fn(|| q.pop_front())
//!     // SAFETY: each token's node is one of the live locals above.
//!     .map(|t| unsafe { (*t.as_ptr()).tid })
//!     .collect();
//! assert_eq!(order, vec![2, 1]);
//! assert!(q.is_empty());
//! ```
//!
//! Rule 1 is no longer something a caller keeps. A node is pushed by giving up its token, and a
//! token is not `Clone`, so the second push of the same node does not compile:
//!
//! ```compile_fail
//! # use core::ptr::NonNull;
//! # use intrusive_fifo::{Fifo, Node, Unqueued};
//! # struct T { next: Option<NonNull<T>> }
//! # unsafe impl Node for T {
//! #     fn next(&self) -> Option<NonNull<Self>> { self.next }
//! #     fn set_next(&mut self, next: Option<NonNull<Self>>) { self.next = next; }
//! # }
//! # let mut b = T { next: None };
//! # let (mut ready, mut inbox) = (Fifo::<T>::new(), Fifo::<T>::new());
//! // SAFETY: `b` is live, on no queue, and has no token yet.
//! let token = unsafe { Unqueued::new(NonNull::from(&mut b)) };
//! ready.push_back(token);
//! inbox.push_back(token); // error[E0382]: use of moved value: `token`
//! ```
//!
//! # What a static analyser sees here, and why it is right (DECISIONS §35)
//!
//! CodeQL flagged the dereferences in [`Fifo::push_back`] and [`Fifo::pop_front`] as
//! `rust/access-invalid-pointer`. **Both alerts are now fixed rather than dismissed**, and the
//! measurement is worth recording because the expectation was wrong: the rule reads as being about
//! pointer validity in general, so the prediction was that moving to [`NonNull`] would improve the
//! code without satisfying the tool. It satisfied the tool outright, main going from two open alerts
//! to zero on this change (`/language:rust`: 2 results on `refs/heads/main`, 0 on `refs/pull/5/head`).
//! The rule was more precise than it looked, and it was pointing at nullness.
//!
//! **Nullness is gone from the contract**: the API takes and returns [`NonNull`] (inside an
//! [`Unqueued`] since milestone 139's round 10), and every caller converts from a reference
//! (`NonNull::from`, never `NonNull::new(..).unwrap()`), so non-nullness is a fact of construction
//! rather than a promise anyone keeps.
//!
//! **Validity is still not expressible**, and no tool result changes that. Rule 2 above says a node outlives its
//! time on the queue, and no type available to us can carry that for a structure whose entire purpose
//! is that the queue does *not* own its nodes. What upholds it is the kernel's state machine: only
//! `Finished` threads are ever freed, a `Finished` thread is on no queue, and one token per node
//! makes "on at most one queue" a type error rather than a corrupted list.
//!
//! So the honest statement is that **this queue is safe because its callers are correct, and nothing
//! in this crate can check that**. The Kani harness below proves the FIFO's *logic* over a symbolic
//! operation sequence against nodes it holds valid by construction; it answers "is the ordering
//! right", never "did a caller free a queued node". Neither tool covers that, and a reader should
//! know it rather than infer safety from two green checkmarks.
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Renamed from
//! `intrusive`: a bare adjective without its noun; the primary type is `Fifo<T: Node>`, and
//! "intrusive" describes a *style* of data structure, not a thing on its own. The term itself is
//! still grounded in the prior art this project learns from, Linux's `list_head` and seL4's TCB
//! queues; only the missing noun was the defect. Introduced 2026-07-23 when the run queues and
//! inboxes went intrusive.

#![cfg_attr(not(test), no_std)]

use core::ptr::NonNull;

/// A type that carries its own queue link.
///
/// # Safety
///
/// `next`/`set_next` must be plain storage: reading back exactly what was stored, touching
/// nothing else. The queue threads its structure through these two methods, so a clever
/// implementation is a corrupted queue.
pub unsafe trait Node: Sized {
    /// Read back the link stored by the most recent [`set_next`](Self::set_next).
    fn next(&self) -> Option<NonNull<Self>>;
    /// Store the link. Plain storage only; see the trait's safety section.
    fn set_next(&mut self, next: Option<NonNull<Self>>);
}

/// **Proof that one node is on no queue**, and the only thing a queue will take.
///
/// Milestone 139, round 10 (calef's ruling C on #1842, 2026-10-08 UTC). Before it, every push was an
/// `unsafe` call whose comment asserted the same sentence, "this node is live and on no other
/// queue", and the kernel's scheduler carried twenty-one hand-written copies of it. A token turns
/// that sentence into ownership: it is not `Clone` or `Copy`, [`Fifo::push_back`] consumes it, and
/// [`Fifo::pop_front`] is the one place a queue hands it back. A node that holds no token cannot be
/// pushed, so a node can no longer be on two queues at once without someone writing `unsafe`.
///
/// `#[repr(transparent)]` over the [`NonNull`] it carries, so it costs nothing at run time: the
/// same register, the same layout, and `Option<Unqueued<T>>` is still one pointer wide.
///
/// What it cannot carry is lifetime (rule 2 in the crate docs). That is the one obligation left,
/// and it is discharged once, where the token is minted ([`Unqueued::new`]), rather than at every
/// push.
///
/// `#[must_use]`, because dropping a token is safe but strands its node: nothing can queue it again
/// without minting a second token, which is exactly the `unsafe` this type exists to keep rare.
///
/// Name: provisional (milestone 139, round 10, 2026-10-08 UTC). `Unqueued` is the name option C on
/// #1842 carried when calef chose that option; whether choosing it ratified the name is his to say.
/// `new`, `as_ptr` and `as_non_null` are this lane's: calef names public items.
#[repr(transparent)]
#[must_use = "a dropped token strands its node: nothing can queue it again without minting another"]
pub struct Unqueued<T>(NonNull<T>);

impl<T> Unqueued<T> {
    /// **Mint the token for `node`.** The one `unsafe` step in queueing.
    ///
    /// # Safety
    ///
    /// - `node` is valid to read and write, and stays valid for as long as this token, or any
    ///   queue it is pushed onto, can still reach it (rule 2).
    /// - `node` is on no queue, and no other `Unqueued` for it exists or will be minted while this
    ///   one, or the queue that consumes it, is alive (rule 1: this is what makes the token unique).
    /// - While a queue holds `node`, nothing but the queue writes its link, and no reference to
    ///   `node` is live across a queue operation (rule 3).
    #[inline(always)]
    pub const unsafe fn new(node: NonNull<T>) -> Self {
        Self(node)
    }

    /// The node, as a pointer. Safe to take: holding a pointer grants nothing, and dereferencing it
    /// is still the caller's `unsafe`.
    #[inline(always)]
    pub const fn as_non_null(&self) -> NonNull<T> {
        self.0
    }

    /// The node, as a raw pointer. See [`as_non_null`](Self::as_non_null).
    #[inline(always)]
    pub const fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }
}

impl<T> PartialEq<NonNull<T>> for Unqueued<T> {
    fn eq(&self, other: &NonNull<T>) -> bool {
        self.0 == *other
    }
}

impl<T> core::fmt::Debug for Unqueued<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Unqueued").field(&self.0).finish()
    }
}

/// The queue: two pointers into nodes it does not own, plus a count.
///
/// FIFO because the scheduler's queues are round-robin: threads leave in the order they arrived.
pub struct Fifo<T: Node> {
    head: Option<NonNull<T>>,
    tail: Option<NonNull<T>>,
    len: usize,
}

// SAFETY: the queue owns no data; it holds nodes only under the caller's contract (rule 3: all
// access serialized by the caller). Same justification as the slab allocator's Send: sharing is
// the caller's problem, solved with a lock or a single-owner rule.
unsafe impl<T: Node> Send for Fifo<T> {}

impl<T: Node> Fifo<T> {
    /// An empty queue.
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
            len: 0,
        }
    }

    /// Whether the queue currently holds no nodes.
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// The number of nodes currently queued. O(1): maintained on every push and pop.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Append a node, consuming its token. Safe: the token is the proof that the node is valid
    /// and on no queue, and taking it by value is what stops the same node being pushed twice.
    pub fn push_back(&mut self, node: Unqueued<T>) {
        let node = node.0;
        // SAFETY: valid and on no queue, which is what `Unqueued::new`'s caller promised for this
        // token; and the token is consumed, so no other push can link this node until a pop.
        unsafe { (*node.as_ptr()).set_next(None) };

        match self.tail {
            // SAFETY: `tail` was pushed earlier with its token and not yet popped, so it is valid.
            Some(tail) => unsafe { (*tail.as_ptr()).set_next(Some(node)) },
            None => self.head = Some(node),
        }
        self.tail = Some(node);
        // Wrapping, because the wrap cannot happen and this is on every IPC (release builds check
        // overflow, notes/overflow-checks.md). `len` counts nodes linked right now, each a distinct
        // live object of nonzero size (it holds a link), so it is bounded by the address space and
        // never reaches `usize::MAX`.
        self.len = self.len.wrapping_add(1);
    }

    /// Detach and return the oldest node, with its token. The returned node's link is cleared: it
    /// leaves the queue carrying no dangling reference into it, and the token says it may be
    /// queued again.
    pub fn pop_front(&mut self) -> Option<Unqueued<T>> {
        let node = self.head?;
        // SAFETY: every node between head and tail was pushed with a token and is still valid
        // (rule 2, the token's minting contract); we are the only accessor (rule 3).
        unsafe {
            self.head = (*node.as_ptr()).next();
            if self.head.is_none() {
                self.tail = None;
            }
            (*node.as_ptr()).set_next(None);
        }
        // Wrapping, because the wrap cannot happen: `head` was `Some`, so a push is outstanding and
        // `len >= 1`. `any_push_pop_interleaving_is_fifo_and_lossless` below holds `len` equal to
        // the model's count after every step, so a decrement that wrapped would fail that proof.
        self.len = self.len.wrapping_sub(1);
        // The token that was consumed at the push, handed back: this node is on no queue now.
        Some(Unqueued(node))
    }
}

impl<T: Node> Default for Fifo<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Machine-checked proofs (DECISIONS §14). The interesting property of a queue is not one push
/// or one pop but what holds across *any* interleaving of them, so the harness drives the real
/// `Fifo` with a symbolic operation sequence and checks it against a trivially-correct model
/// (an array and two counters). Every push/pop pattern up to the bound is covered at once,
/// including the regrowth-from-empty transitions where head/tail bugs live.
#[cfg(kani)]
mod verification {
    use super::*;

    struct N {
        next: Option<NonNull<N>>,
        tag: usize,
    }

    // SAFETY: `next` and `set_next` are plain field storage and touch nothing else, which is the
    // whole of the `Node` contract.
    unsafe impl Node for N {
        fn next(&self) -> Option<NonNull<Self>> {
            self.next
        }
        fn set_next(&mut self, next: Option<NonNull<Self>>) {
            self.next = next;
        }
    }

    /// **FIFO order, no loss, no invention, across every operation sequence.** Six symbolic
    /// steps over three nodes: each step pushes some not-currently-queued node or pops. The
    /// model records the order tags went in; the queue must hand them back in exactly that
    /// order, agree about emptiness and length at every step, and never dereference a stale
    /// link (which Kani would report as an invalid pointer access).
    /// Falsification: replayable `crates/intrusive_fifo/falsifications/verification.any_push_pop_interleaving_is_fifo_and_lossless.patch`
    #[kani::proof]
    fn any_push_pop_interleaving_is_fifo_and_lossless() {
        let mut nodes = [
            N { next: None, tag: 0 },
            N { next: None, tag: 1 },
            N { next: None, tag: 2 },
        ];
        // One token per node, minted from references, so the harness proves the same API the
        // kernel calls. A token is in `held` while its node is off the queue and in the queue while
        // it is on it, exactly as in the kernel.
        //
        // SAFETY: three distinct live locals declared before `q`, each on no queue, each minted
        // once, and the harness is the only accessor.
        let mut held: [Option<Unqueued<N>>; 3] = unsafe {
            [
                Some(Unqueued::new(NonNull::from(&mut nodes[0]))),
                Some(Unqueued::new(NonNull::from(&mut nodes[1]))),
                Some(Unqueued::new(NonNull::from(&mut nodes[2]))),
            ]
        };

        let mut q: Fifo<N> = Fifo::new();

        // The model: a ring of tags in push order, and which nodes are currently queued.
        let mut model = [usize::MAX; 6];
        let (mut m_head, mut m_tail) = (0usize, 0usize);
        let mut queued = [false; 3];

        for _ in 0..6 {
            let choice: usize = kani::any();
            kani::assume(choice <= 3);
            if choice < 3 {
                // The token is the check now: a node is pushed exactly when its token is held.
                if let Some(token) = held[choice].take() {
                    assert!(!queued[choice], "a token was held for a queued node");
                    queued[choice] = true;
                    model[m_tail] = choice;
                    m_tail += 1;
                    q.push_back(token);
                }
            } else {
                let popped = q.pop_front();
                if m_head == m_tail {
                    assert!(popped.is_none(), "popped from an empty queue");
                } else {
                    let expect = model[m_head];
                    m_head += 1;
                    queued[expect] = false;
                    let token = popped.expect("lost a node");
                    // SAFETY: pop returns only nodes we pushed, all still valid.
                    let got = unsafe { (*token.as_ptr()).tag };
                    assert_eq!(got, expect, "not FIFO");
                    held[expect] = Some(token);
                }
            }
            assert_eq!(q.len(), m_tail - m_head);
            assert_eq!(q.is_empty(), m_head == m_tail);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct N {
        next: Option<NonNull<N>>,
        tag: u32,
    }

    // SAFETY: `next` and `set_next` are plain field storage and touch nothing else, which is the whole of the `Node` contract.
    unsafe impl Node for N {
        fn next(&self) -> Option<NonNull<Self>> {
            self.next
        }
        fn set_next(&mut self, next: Option<NonNull<Self>>) {
            self.next = next;
        }
    }

    fn node(tag: u32) -> Box<N> {
        Box::new(N { next: None, tag })
    }

    /// Mint `n`'s token. Every test calls this once per node, on a `Box` declared before its
    /// queue, so the node outlives the queue (locals drop in reverse) and is on no queue yet.
    fn token(n: &mut Box<N>) -> Unqueued<N> {
        // SAFETY: a live boxed node on no queue, minted once per test (see above); the test is the
        // only accessor.
        unsafe { Unqueued::new(NonNull::from(&mut **n)) }
    }

    fn tag(t: &Unqueued<N>) -> u32 {
        // SAFETY: every token in these tests names one of the test's live boxed nodes.
        unsafe { (*t.as_ptr()).tag }
    }

    #[test]
    fn fifo_order() {
        let (mut a, mut b, mut c) = (node(1), node(2), node(3));
        let mut q: Fifo<N> = Fifo::new();
        assert!(q.is_empty());

        q.push_back(token(&mut a));
        q.push_back(token(&mut b));
        q.push_back(token(&mut c));
        assert_eq!(q.len(), 3);

        // A non-empty queue says so. Milestone 85: every earlier assertion met is_empty only in
        // the empty state, so a body stuck at `true` passed.
        assert!(!q.is_empty());

        // len falls by exactly one per pop. The push side was asserted above; the pop side was
        // not, and the scheduler trusts this counter for its run-queue accounting.
        let first = q.pop_front().map(|t| tag(&t));
        assert_eq!(first, Some(1));
        assert_eq!(q.len(), 2);

        let tags: Vec<u32> = core::iter::from_fn(|| q.pop_front())
            .map(|t| tag(&t))
            .collect();
        assert_eq!(tags, vec![2, 3]);
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    /// The empty-again transitions: a queue drained to empty accepts new nodes correctly (the
    /// classic tail-pointer bug), and a popped node can be pushed again with the token its pop
    /// handed back.
    #[test]
    fn drain_then_reuse() {
        let (mut a, mut b) = (node(1), node(2));
        let mut q: Fifo<N> = Fifo::new();

        q.push_back(token(&mut a));
        let ta = q.pop_front().unwrap();
        assert_eq!(tag(&ta), 1);
        assert!(q.pop_front().is_none());

        q.push_back(token(&mut b));
        q.push_back(ta); // the token the pop handed back is what makes queueing `a` again legal
        assert_eq!(q.pop_front().map(|t| tag(&t)), Some(2));
        assert_eq!(q.pop_front().map(|t| tag(&t)), Some(1));
        assert!(q.is_empty());
    }

    /// The token is the pointer and nothing more: `#[repr(transparent)]` is the zero-cost claim the
    /// kernel's footprint gate rests on, and `Option` of one is still one word.
    #[test]
    fn a_token_is_one_pointer_wide() {
        use core::mem::size_of;
        assert_eq!(size_of::<Unqueued<N>>(), size_of::<NonNull<N>>());
        assert_eq!(size_of::<Option<Unqueued<N>>>(), size_of::<NonNull<N>>());
    }

    /// A popped node leaves with a clean link: it does not secretly point back into the queue.
    #[test]
    fn a_popped_node_carries_no_link() {
        let (mut a, mut b) = (node(1), node(2));
        let mut q: Fifo<N> = Fifo::new();
        q.push_back(token(&mut a));
        q.push_back(token(&mut b));
        let p = q.pop_front().unwrap();
        // SAFETY: `p` was just popped, so it is live and unlinked; that it is unlinked is what this test asserts.
        assert!(unsafe { (*p.as_ptr()).next() }.is_none());
    }
}
