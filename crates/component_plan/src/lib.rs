//! **`component_plan`: what a component declares it needs, and how a supervisor wires it**
//! (milestone 23, DECISIONS §41).
//!
//! A component is a swappable, vendor-shippable unit behind a stable contract. §41 says what makes
//! one substitutable: *any program that speaks the protocol and holds the right capabilities **is**
//! the component*. The protocol half of that sentence has always been in a crate. The capability
//! half was a set of literal arrays in the operator's own source, which is the defect milestone 23's
//! roadmap block names in six words: **"endowments are literals in the operator's source."** This
//! crate is the capability half, lifted out to where a declaration can be read, checked, and proved.
//!
//! It performs no IO and makes no syscalls. It turns two declarations into a decision: a
//! [`Requirements`] (what the component says it needs) plus a [`Provisions`] (what the supervisor
//! will route to *this* child) into a [`Plan`] (exactly the capabilities and mappings to endow) or a
//! typed [`Refusal`]. See notes/component-manifest.md.
//!
//! # It is not `grant_plan::Manifest`, and the difference is one sentence
//!
//! [`grant_plan::Manifest`](../grant_plan/struct.Manifest.html) declares **what a human at a prompt
//! may designate** for a program; [`Requirements`] declares **what a supervisor must route before a
//! component can serve anyone**. Four things follow from that, and they are why this is a sibling
//! rather than a subtype:
//!
//! - **There is no command line and no human.** `grant_plan`'s thesis is that designation is
//!   authorization: a person names a file and the naming is the grant. A component is started by a
//!   supervisor with nothing typed, so every field about placing a typed token (`arg`, `flags`,
//!   `file`, `dir`) has no meaning here, and there is no prompt at which to refuse.
//! - **The vocabularies do not overlap.** Nothing in `grant_plan::Manifest` can name an rendezvous, a
//!   device, or a shared page; nothing here can name an argument or a short option.
//! - **A component *serves*, and a program only consumes.** [`Direction`] is the axis this crate
//!   adds, and it is the axis §41's central refusal test turns on: `Serve` is `READ` on an rendezvous
//!   and `Use` is `WRITE`, so a client of the stable rendezvous cannot become its server. `grant_plan`
//!   has no field that could carry that, and no program it spawns would ever set one.
//! - **The keys are different, and this is the one that decides drop-in.** `grant_plan::Manifest` is
//!   reachable only through `Prog`, a closed enum with a static table compiled into the shell, so
//!   admitting a new component would mean editing the shell. [`Requirements`] is a value the
//!   *contract* owns, so a build that speaks the contract is wired by code that never heard of it.
//!
//! What the two **do** share is a shape, and the shape is the reusable idea rather than the struct:
//! a declaration, plus what the wirer holds, checked into an endowment or a typed refusal, before
//! anything is spawned. `grant_plan::plan` and [`plan`] are the same function over different worlds.
//!
//! # A manifest is a request; the provisions are the authority
//!
//! This is Fuchsia's `use`/`offer` split, and it is what keeps a vendor's manifest from being a
//! privilege-escalation surface. A component names its needs, and it can name anything it likes. The
//! supervisor decides, per child, which of its own objects each name resolves to, and a name it did
//! not list resolves to nothing: [`Refusal::Unprovided`], **before the component is built**. So a
//! manifest cannot widen a component's authority; it can only fail to be satisfiable.
//!
//! The corollary is the property that makes a component substitutable at all: **the name is the
//! component's and the object is the supervisor's.** `chatty`'s manifest asks to *use* an rendezvous it
//! calls `service`; on the direct channel the operator routes that to the shared service rendezvous,
//! and on the queued channel it routes the same name to the broker's front rendezvous. One
//! declaration, two wirings, and the component cannot tell which it got.
//!
//! # Structure is a compile error; provisioning is a runtime refusal
//!
//! The two halves are checked on different rungs of the ladder, deliberately.
//!
//! A malformed *declaration* (a role declared twice, two mappings at one address, a component that
//! asks to exist in zero pages) is caught by [`Requirements::problem`], which is a `const fn`. Every
//! real manifest in this tree asserts on it in a `const` item, so a malformed one **does not
//! compile**, on both architectures, without any test having to run.
//!
//! A *provisioning* mismatch cannot be caught that way, because what a supervisor holds is only
//! known when it runs. That is [`plan`]'s job, and its answer is a value the supervisor reports.
//!
//! # EXAMPLES
//!
//! A component declares two capabilities and one page, and a supervisor wires it:
//!
//! ```
//! use component_plan::{CapNeed, Direction, MapNeed, PageKind, Provisions, Requirements};
//!
//! /// The contract's capability half. Lives beside the contract's wire half, not in the operator.
//! pub const TALKER: Requirements = Requirements {
//!     contract: "talker",
//!     caps: &[
//!         CapNeed { role: "service", direction: Direction::Serve },
//!         CapNeed { role: "report", direction: Direction::Use },
//!     ],
//!     maps: &[MapNeed { role: "witness", va: 0x0300_0000, kind: PageKind::Shared }],
//!     pages: 32,
//!     depends_on: &[],
//!     handoff: None,
//! };
//!
//! // Malformed declarations do not compile. This one is well formed, so the assertion holds.
//! const _: () = assert!(TALKER.problem().is_none());
//!
//! // The slot a component reads is derived from its own declaration, so the two cannot drift.
//! pub const SERVICE: u64 = component_plan::slot_of(&TALKER, "service");
//! assert_eq!(SERVICE, 0);
//!
//! // The supervisor routes each name to one of its own slots, per child.
//! let plan = component_plan::plan(
//!     &TALKER,
//!     &Provisions { held: &[("service", 5), ("report", 1), ("witness", 9)] },
//! )
//! .expect("every declared role is routed");
//!
//! // Rights come from the declared direction. The supervisor never spells one.
//! assert_eq!(plan.caps(), &[(5, abi::rights::READ), (1, abi::rights::WRITE)]);
//! assert_eq!(plan.maps(), &[(0x0300_0000, 9, abi::address_space::MAP_RW)]);
//! assert_eq!(plan.pages(), 32);
//! ```
//!
//! A supervisor that does not hold what the component asks for is refused, and nothing is built:
//!
//! ```
//! # use component_plan::{CapNeed, Direction, Provisions, Refusal, Requirements};
//! # const TALKER: Requirements = Requirements {
//! #     contract: "talker",
//! #     caps: &[CapNeed { role: "service", direction: Direction::Serve }],
//! #     maps: &[],
//! #     pages: 32,
//! #     depends_on: &[],
//! #     handoff: None,
//! # };
//! let refused = component_plan::plan(&TALKER, &Provisions { held: &[("report", 1)] });
//! assert_eq!(refused, Err(Refusal::Unprovided { role: "service" }));
//! ```
//!
//! # BUGS
//!
//! **A manifest is compiled in, not shipped with the build.** So a vendor cannot
//! today hand over a binary and have its needs read out of it; what this crate removes is the
//! endowment from the *operator*, not from the tree. calef ruled on 2026-09-26 that a manifest
//! travels inside the executable as an ELF note, per DECISIONS §197 (a package is one archive file);
//! the note's owner, type and encoding are still the architect's, and nothing reads a note yet.
//! See notes/component-manifest.md.
//!
//! **[`Requirements::pages`] is a property of the build, not of the contract**, and it is the only
//! field here that is. How many pages an instance needs depends on its image size, its stack and its
//! page tables, so two vendors' builds of one contract can honestly differ. Declaring it per contract
//! is right only while every build of a contract fits the same number, which is true in this tree and
//! is not true in general. It is the strongest single argument for eventually shipping the manifest
//! with the binary.
//!
//! **A [`Handoff`] is one page, so a blob is at most 4 KiB.** §209 (state handoff is an opaque blob over a granted frame, and it is optional) says "a granted shared `Frame`",
//! singular, and the one stateful component in the swap suite needs sixteen bytes. The component
//! §209 was reopened for is `redoxfs_server`, whose open-handle table and cache will not fit in a
//! page, so the field will want a page count (or a frame run, per §102 (a frame names a run)) the day that server declares
//! one. Not added now because nothing would exercise the second page.
//!
//! **A malformed declaration's compile error names the manifest, not the role.** `const` evaluation
//! reports an assertion that failed, so the reader is told which `Requirements` is wrong and has to
//! look for the duplicate themselves. [`Requirements::problem`] returns the [`Refusal`] with the role
//! in it, and a host test that calls it prints the good message; the `const` assertion cannot.
//!
//! **[`MAX_CAPS`] and [`MAX_MAPS`] are fixed, and a component past them is refused rather than
//! served.** There is no allocator here and a [`Plan`] is a value, so the bound is a real limit and
//! not a hint. Eight and four are roughly double what any manifest in this tree asks for.
//!
//! **Nothing checks that a supervisor's routed object is the *kind* the role wants.** A supervisor
//! that routes a frame where the component declared an rendezvous gets a plan, and the kernel refuses
//! the `CAP_INSERT` or the component's first `RECV` instead. The declaration carries the direction
//! and the address, which is what a supervisor can get wrong silently; the object type is what the
//! kernel already checks loudly.
//!
//! Name: provisional, and ruled: calef ruled **`capability_plan`** on 2026-09-13, working the
//! unratified worklist. The block stays `provisional` because the ratified name is not this
//! crate's until the rename is performed, and until then `component_plan` belongs on the worklist
//! rather than off it. Minted by milestone 23's lane.
//!
//! **`component` named the customer; the crate is about capabilities.** calef, shown the old name
//! and the two alternatives on offer: *"Neither component nor substitution plan convey to me in
//! the name what this thing is. The word capability seems important to it."* It is, and this
//! header says so four lines up: §41's test is that *any program that speaks the protocol and
//! holds the right capabilities is the component*, the protocol half has always been a crate, and
//! **this crate is the capability half**, lifted out of the literal arrays milestone 23's block
//! calls "endowments are literals in the operator's source".
//!
//! **The argument that lost, recorded because it is a real cost rather than a bad idea.** A grant
//! *is* a capability, so `capability_plan` and `grant_plan` read as near-synonyms while the four
//! bullets above insist they are siblings rather than one being a subtype of the other. That
//! distinction is now the header's to carry rather than the name's, which is where it already
//! was. `supervision_plan` was the alternative that kills the synonymy by naming who wires (a
//! shell at a prompt, against a supervisor with nothing typed), and it was refused for dropping
//! the word that makes the crate legible.
//!
//! **A second reason the old name had to go, which arrived on its own.** Milestone 175 landed on
//! 2026-09-13 and split `user/` into `components/` and `fixtures/`, so `component` now also names
//! a directory. Those are different senses: §41's component is a contract a program satisfies,
//! 175's is a place a file sits, and nothing stops a program in `fixtures/` from having one of
//! these plans. The collision did not decide the ruling and would have needed recording either way.
//!
//! Refused `component_manifest` and `manifest` (a second `Manifest` in a tree that already has
//! `grant_plan::Manifest` is the comprehension disaster this crate's own header argues against),
//! and `capability_declaration` (a declaration is only half of it; the wiring is the other half and
//! is where the checking lives, which is why the ratified name keeps `plan`).

#![no_std]

/// **How many capabilities one component may declare.** A [`Plan`] is a value in a `no_std` program
/// with no allocator, so this is a hard bound rather than a hint: a manifest past it is
/// [`Refusal::TooManyCaps`].
///
/// Eight, which is roughly double the four that the widest manifest in this tree asks for, and which
/// leaves room below `abi::fault::FAULT_EP_SLOT` for the placed slots a component might also want.
pub const MAX_CAPS: usize = 8;

/// **How many pages one component may declare.** [`MAX_CAPS`]'s twin, for the mapping half. Four,
/// against the two that the widest manifest here asks for.
pub const MAX_MAPS: usize = 4;

/// **Which way an rendezvous points**, which is the axis this crate adds over a program's manifest and
/// the reason a component's declaration is a different object from a program's.
///
/// It is a required field with no default, and that is deliberate in `writes_while_reading`'s shape
/// (AGENTS.md's rung one): a component **cannot declare an rendezvous without saying whether it
/// answers on it**. The rights fall out of the answer, so the supervisor never spells `READ` or
/// `WRITE` and cannot spell one wrong.
///
/// The stakes are §41's central refusal. `SEND` and `RECV` are gated by different rights on the same
/// object, so the same rendezvous handed out two ways is a one-way pipe in whichever direction each
/// holder was trusted with. A typo that gave a client `READ` would let it park in `RECV_CAP` and take
/// its own server's requests, and the test that catches that
/// (`a_client_of_the_stable_rendezvous_cannot_become_its_server`) would fail for a reason no reader
/// could see from the operator's source. There is now nothing to typo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    /// **The component answers here.** Requests arrive on this rendezvous and it replies: `READ`.
    Serve,
    /// **The component asks here, and only asks.** It sends and never receives: `WRITE`.
    Use,
}

impl Direction {
    /// The rights a child is endowed with for an rendezvous it declared this way. The only place in
    /// the tree a component's rendezvous rights are decided.
    pub const fn rights(self) -> u64 {
        match self {
            Direction::Serve => abi::rights::READ,
            Direction::Use => abi::rights::WRITE,
        }
    }
}

/// **One capability a component declares it needs**, named by the component and routed by the
/// supervisor.
///
/// The `role` is the component's own word for what it needs the object *for*, never the
/// supervisor's word for the object. That is what lets one declaration be wired two ways: `chatty`
/// asks to use `service`, and whether that is the shared rendezvous or a queue broker's front rendezvous
/// is the supervisor's business and invisible to the component.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CapNeed {
    /// The component's name for this need. Unique within one [`Requirements`]; a repeat is
    /// [`Refusal::DuplicateRole`] at compile time.
    pub role: &'static str,
    /// Which way it points. See [`Direction`] for why this has no default.
    pub direction: Direction,
}

/// **What kind of page a mapping need is for.** Two kinds, because the two behave differently at the
/// one moment a live replacement cares about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageKind {
    /// Ordinary memory the supervisor shares with the component, mapped read/write.
    Shared,
    /// A device's registers.
    ///
    /// Called out as its own kind for one reason, and it is §41's: revocation is by physical page, so
    /// a device the supervisor is about to take back **cannot be installed at build time**. A plan
    /// sorts these last and [`Plan::devices`] hands them over separately, so the operator can endow
    /// everything else early and map the registers on the far side of the revoke, which is what keeps
    /// the down window four syscalls wide.
    DeviceRegisters,
}

impl PageKind {
    /// The mapping mode this kind is installed with. Device registers are mapped device-typed
    /// read/write by the kernel whatever mode is asked for, so the mode a device need carries is
    /// the conservative one rather than a claim.
    pub const fn mode(self) -> u64 {
        match self {
            PageKind::Shared => abi::address_space::MAP_RW,
            PageKind::DeviceRegisters => abi::address_space::MAP_RO,
        }
    }
}

/// **One page a component declares it needs, and the address it needs it at.**
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MapNeed {
    /// The component's name for this need, routed exactly as a [`CapNeed`]'s is.
    pub role: &'static str,
    /// The virtual address the component reads it at. It is in the declaration rather than in the
    /// supervisor because it is a fact about the component's own code: the component compiles the
    /// address into a `read_volatile`, and a supervisor that mapped it somewhere else would produce
    /// a component that faults for a reason a long way from the mistake.
    pub va: u64,
    /// Shared memory or a device's registers. See [`PageKind`].
    pub kind: PageKind,
}

/// **The role name a supervisor routes a component's handoff page under** (DECISIONS §209). One fixed
/// word rather than a role the component picks, because a handoff page is not a need the component
/// names for its own purposes: it is the one page a supervisor must route to the **same** frame in the
/// outgoing and the incoming instance, and a supervisor wiring two builds of one contract has to be
/// able to find it without reading either build's code.
///
/// Name: provisional, minted on 2026-09-26 by the lane for milestone 23 (a capability-routed
/// component OS with live replacement), beside §209's own provisional `handoff` field. An
/// architect's call.
pub const HANDOFF_ROLE: &str = "handoff";

/// **Where a component that carries state across a live replacement wants its handoff page**
/// (DECISIONS §209: state handoff is an opaque blob over a granted frame, and it is optional).
///
/// The page's bytes are the component's business and nobody else's. The supervisor never reads them
/// and this crate never names their shape, which is the refusal of §116 (live component state handoff is declined, for want of a customer) to design one wire format for
/// every kind of live state, kept. What the declaration buys the supervisor is two facts it could not
/// otherwise know: **that** this contract carries state (so a swap must wait for the incoming
/// instance to say it absorbed it before the outgoing one is retired), and **where** both instances
/// read and write it (so the supervisor can route one frame to both).
///
/// One page. See this crate's `BUGS` for why that is a limit and not a design.
///
/// Name: provisional (milestone 23's lane, 2026-09-26). §209 left the field name `handoff`
/// provisional and did not take one for the type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Handoff {
    /// The virtual address both instances read and write the blob at. In the declaration for
    /// [`MapNeed::va`]'s reason: it is compiled into the component's own code.
    pub va: u64,
}

/// **What a component declares it needs.** The capability half of a contract, beside the wire half.
///
/// The order of [`caps`](Requirements::caps) is load-bearing: it **is** the component's capability table slot
/// order, because `supervision_protocol::ChildEndowment::caps` lands its entries in slots 0, 1, 2, ...
/// Before this crate that agreement was a comment in two files. Now the component derives its own
/// slot numbers from this list with [`slot_of`], so a reordered manifest is a compile error rather
/// than a component reading the wrong slot at run time.
///
/// What is **not** here is as deliberate as what is. Supervision is not declared, because a
/// component does not choose whether it is watched: that is the supervisor's decision and it is done
/// *to* the component (§32). Start arguments are not declared either, because they are
/// configuration rather than authority: which log entries this instance writes and whether it was
/// handed a device are facts about the wiring, and the component is told them rather than asking.
/// A manifest declares what a component **needs**, not everything about how it is started.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Requirements {
    /// The contract this component implements, for a reader and for a diagnostic. Not matched
    /// against anything: what makes a build substitutable is that it speaks the protocol, and a
    /// string it carries cannot prove that.
    pub contract: &'static str,
    /// The capabilities it needs, **in capability table slot order**. See the struct docs.
    pub caps: &'static [CapNeed],
    /// The pages it needs mapped, each at the address its own code reads.
    pub maps: &'static [MapNeed],
    /// How many pages of the supervisor's budget one instance is built out of: its segments, its
    /// stack, its address-space root, its TCB, its page tables and its revocation log.
    ///
    /// The one field here that is a property of the **build** rather than of the contract, which is
    /// the crate's second `BUGS` entry and the strongest argument for eventually shipping a manifest
    /// with the binary that satisfies it.
    pub pages: u64,
    /// **Which contracts this component cannot silently tolerate the absence of** (milestone 23's
    /// dependency-aware orchestration residual). Named by contract, not by role: a role name is
    /// resolved to an object by the supervisor's [`Provisions`] and that resolution can vary by
    /// wiring (`CLIENT`'s `service` role is a console on the direct channel and a broker's front
    /// rendezvous on the queued one), so a role cannot say which contract will answer it. A contract
    /// name can, and does not change across wirings.
    ///
    /// **Populated only for a component that itself serves others**, and empty otherwise, because
    /// that is the whole of what this field is for: deciding who [`dependents`] must warn before a
    /// swap. A pure consumer (no `Direction::Serve` need at all, like `CLIENT`) never needs warning.
    /// It calls through a `CALL`, and DECISIONS §41 already proved that call degrades for free: a
    /// `CALL` that finds nobody receiving parks on the rendezvous's own sender queue, and the next
    /// server to `RECV_CAP` drains it. Nothing has to tell a pure consumer anything, so nothing here
    /// claims one needs telling. A component that *serves* others while itself calling through to a
    /// swappable dependency is different: `broker` blocks its one serving thread on a `CALL` to its
    /// backend, so if the backend is what is being swapped, `broker` cannot go on answering its own
    /// producers unless it is told first (`BOP_DOWN`) to stop calling through and start buffering.
    /// That is the edge this field exists to name.
    ///
    /// **Direct dependents only.** A dependent that has its own decoupling mechanism (a queue, a
    /// cache) is not assumed to propagate the warning to *its* dependents, because whether it needs
    /// to is a property of that mechanism and not something this crate can infer from two contract
    /// names. See [`dependents`]'s docs and this crate's `BUGS` for what that leaves unbuilt.
    pub depends_on: &'static [&'static str],
    /// **Whether this contract carries state across a live replacement, and where** (DECISIONS §209,
    /// milestone 23's last residual). `None` is the default answer and the common one: a component
    /// that declares nothing is killed and replaced, which is what every component in this tree was
    /// before §209 and what roughly fifty of fifty-two still are.
    ///
    /// A required field with no default, for [`Direction`]'s reason: a contract **cannot be declared
    /// without saying whether it hands state off**, so "we forgot" is not a state the tree can be in.
    ///
    /// `Some` does two things. [`plan`] routes the supervisor's [`HANDOFF_ROLE`] object to the page
    /// at [`Handoff::va`], and refuses with [`Refusal::Unprovided`] when the supervisor routes
    /// nothing there. That refusal is the point: **a supervisor that cannot carry state must not
    /// silently kill-and-replace a component that declared some**, because the loss would look
    /// exactly like a successful swap. And [`Plan::handoff`] tells the supervisor to wait for the
    /// incoming instance's verdict before it retires the outgoing one, since under §209 a swap
    /// whose state was not absorbed does not commit.
    ///
    /// Name: provisional, per §209, which names it and does not ratify it.
    pub handoff: Option<Handoff>,
}

impl Requirements {
    /// **Whether this declaration is well formed at all**, independent of any supervisor.
    ///
    /// A `const fn`, and that is the point: every real manifest in this tree asserts on it in a
    /// `const` item, so a malformed declaration does not compile. Three things are checked, and each
    /// one is a mistake that would otherwise be silent:
    ///
    /// - **A role declared twice.** The component would be handed two slots and read one of them,
    ///   and nothing at run time distinguishes that from a correct wiring.
    /// - **Two pages at one address.** One mapping silently wins; which one depends on the order the
    ///   supervisor happened to install them.
    /// - **Zero pages.** A component nothing pays for cannot be built, and the failure would arrive
    ///   as a region split refusing for no visible reason.
    ///
    /// Over-long declarations are **not** checked here, because [`MAX_CAPS`] is this crate's limit
    /// rather than the declaration's defect: it is [`plan`]'s refusal, where a reader meets the
    /// bound.
    pub const fn problem(&self) -> Option<Refusal> {
        if self.pages == 0 {
            return Some(Refusal::NoPages);
        }
        let mut i = 0;
        while i < self.caps.len() {
            let mut j = i + 1;
            while j < self.caps.len() {
                if str_eq(self.caps[i].role, self.caps[j].role) {
                    return Some(Refusal::DuplicateRole {
                        role: self.caps[i].role,
                    });
                }
                j += 1;
            }
            i += 1;
        }
        let mut i = 0;
        while i < self.maps.len() {
            let mut j = i + 1;
            while j < self.maps.len() {
                if str_eq(self.maps[i].role, self.maps[j].role) {
                    return Some(Refusal::DuplicateRole {
                        role: self.maps[i].role,
                    });
                }
                if self.maps[i].va == self.maps[j].va {
                    return Some(Refusal::OverlappingVa {
                        va: self.maps[i].va,
                    });
                }
                j += 1;
            }
            i += 1;
        }
        // The handoff page is one more mapping, so it is held to the two rules every mapping is:
        // its role word may not be declared a second time, and its address may not be shared.
        if let Some(h) = self.handoff {
            let mut i = 0;
            while i < self.caps.len() {
                if str_eq(self.caps[i].role, HANDOFF_ROLE) {
                    return Some(Refusal::DuplicateRole { role: HANDOFF_ROLE });
                }
                i += 1;
            }
            let mut i = 0;
            while i < self.maps.len() {
                if str_eq(self.maps[i].role, HANDOFF_ROLE) {
                    return Some(Refusal::DuplicateRole { role: HANDOFF_ROLE });
                }
                if self.maps[i].va == h.va {
                    return Some(Refusal::OverlappingVa { va: h.va });
                }
                i += 1;
            }
        }
        None
    }

    /// How many pages a plan of this declaration maps: its declared pages, plus the handoff page if
    /// it declares one.
    const fn mapped(&self) -> usize {
        self.maps.len() + if self.handoff.is_some() { 1 } else { 0 }
    }
}

/// **What a supervisor will route to one child**, as its own slot per role name.
///
/// Per child rather than per supervisor, and that is Fuchsia's routing rather than an ambient
/// environment: the same supervisor hands the same role name to two children and may resolve it to
/// two different objects, which is exactly what makes a component's peer substitutable. A role the
/// supervisor does not list resolves to nothing, so a manifest can never widen its own authority.
///
/// A supervisor may list roles nothing needs. That is not an error: it holds more than any one child
/// requires, and refusing the surplus would make one table per child mandatory for no gain.
#[derive(Clone, Copy, Debug)]
pub struct Provisions<'a> {
    /// `(role name, the supervisor's own capability table slot holding the object)`.
    pub held: &'a [(&'a str, u64)],
}

impl<'a> Provisions<'a> {
    /// The slot this supervisor routes `role` to, or `None`.
    pub fn slot_for(&self, role: &str) -> Option<u64> {
        let mut i = 0;
        while i < self.held.len() {
            if self.held[i].0 == role {
                return Some(self.held[i].1);
            }
            i += 1;
        }
        None
    }
}

/// **Why a component could not be wired.** A value the supervisor reports, never a panic: a
/// supervisor that cannot satisfy a manifest has a legible thing to say and a child it did not
/// build.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// The supervisor routes nothing to a role the component declared. **The security-relevant
    /// one**: it is what stops a manifest from being a request the supervisor is obliged to honour.
    Unprovided {
        /// The component's name for the need that went unrouted.
        role: &'static str,
    },
    /// One role name appears twice in a declaration.
    DuplicateRole {
        /// The name that repeats.
        role: &'static str,
    },
    /// Two pages are declared at one virtual address.
    OverlappingVa {
        /// The address they collide at.
        va: u64,
    },
    /// The declaration asks for more capabilities than [`MAX_CAPS`].
    TooManyCaps {
        /// How many it asked for.
        asked: usize,
    },
    /// The declaration asks for more pages than [`MAX_MAPS`].
    TooManyMaps {
        /// How many it asked for.
        asked: usize,
    },
    /// The declaration asks to exist in zero pages.
    NoPages,
}

impl Refusal {
    /// **One word, so a refusal can be reported over IPC.** A supervisor inside a test has two
    /// registers and no way to send a string, and a refusal that only exists as a `&'static str` in
    /// a dead process is a refusal nothing can assert on.
    pub const fn code(self) -> u64 {
        match self {
            Refusal::Unprovided { .. } => 1,
            Refusal::DuplicateRole { .. } => 2,
            Refusal::OverlappingVa { .. } => 3,
            Refusal::TooManyCaps { .. } => 4,
            Refusal::TooManyMaps { .. } => 5,
            Refusal::NoPages => 6,
        }
    }

    /// What a person is told. `grant_plan::Refusal::message`'s convention: a sentence about this
    /// system's model, not a Unix errno translated.
    pub const fn message(self) -> &'static str {
        match self {
            Refusal::Unprovided { .. } => "this supervisor routes nothing to a role it declares",
            Refusal::DuplicateRole { .. } => "it declares one role name twice",
            Refusal::OverlappingVa { .. } => "it declares two pages at one address",
            Refusal::TooManyCaps { .. } => "it declares more capabilities than a plan can carry",
            Refusal::TooManyMaps { .. } => "it declares more pages than a plan can carry",
            Refusal::NoPages => "it declares no memory to be built in",
        }
    }
}

/// **Exactly what to endow one child with**, in the form `supervision_protocol::ChildEndowment` takes.
///
/// A value with no allocation, bounded by [`MAX_CAPS`] and [`MAX_MAPS`], so a supervisor holds it on
/// its stack and hands out slices of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Plan {
    caps: [(u64, u64); MAX_CAPS],
    ncaps: usize,
    maps: [(u64, u64, u64); MAX_MAPS],
    nmaps: usize,
    ndevices: usize,
    pages: u64,
    handoff: Option<u64>,
}

impl Plan {
    /// `(the supervisor's slot, the child's rights)` per declared capability, **in declared order**,
    /// which is the child's slot order. Feeds `ChildEndowment::caps` directly.
    pub fn caps(&self) -> &[(u64, u64)] {
        &self.caps[..self.ncaps]
    }

    /// `(child va, the supervisor's slot, mode)` per declared page. Feeds `ChildEndowment::maps`.
    ///
    /// Device registers sort last. See [`maps_without_devices`](Plan::maps_without_devices) for the
    /// reason and for the swap that needs it.
    pub fn maps(&self) -> &[(u64, u64, u64)] {
        &self.maps[..self.nmaps]
    }

    /// **Everything but the device registers**, for a supervisor that is about to revoke a device.
    ///
    /// §41: revocation is by physical page, so a replacement endowed with the registers before the
    /// revoke would lose its copy to the revoke, and since the kernel mints a device capability once
    /// at boot nothing could hand one back. A supervisor building a replacement therefore endows it
    /// with this, revokes, and then installs [`devices`](Plan::devices) into the child's address
    /// space. That is what keeps the down window four syscalls wide.
    pub fn maps_without_devices(&self) -> &[(u64, u64, u64)] {
        &self.maps[..self.nmaps - self.ndevices]
    }

    /// The device mappings [`maps_without_devices`](Plan::maps_without_devices) left out, to be
    /// installed on the far side of a revoke.
    pub fn devices(&self) -> &[(u64, u64, u64)] {
        &self.maps[self.nmaps - self.ndevices..self.nmaps]
    }

    /// How many pages of the supervisor's budget one instance costs.
    pub fn pages(&self) -> u64 {
        self.pages
    }

    /// **Where this component's handoff page is, if it carries state across a swap** (DECISIONS
    /// §209). `None` means kill-and-replace: retire the outgoing instance as soon as the incoming one
    /// is running. `Some` means the swap does not commit until the incoming instance says it absorbed
    /// the blob, and a supervisor that retires the outgoing instance first has thrown the state away
    /// with nothing to show for it.
    ///
    /// The mapping itself is already in [`maps`](Plan::maps), among the shared pages and installed
    /// at build time in both instances, so a supervisor never installs it by hand. Unlike a device it
    /// is not deferred past a revoke, and it cannot be: `PageFrame::REVOKE` is symmetric and would take
    /// the supervisor's own capability with everyone else's. The outgoing instance is quiesced and
    /// blocked before the incoming one is started, so the two never run against the page at once.
    ///
    /// Name: provisional (milestone 23's lane, 2026-09-26).
    pub fn handoff(&self) -> Option<u64> {
        self.handoff
    }
}

/// **Check a component's declaration against what a supervisor will route to it**, and yield the
/// endowment or say why not.
///
/// This is the whole of the mechanism, and every check in it is one a supervisor would otherwise
/// have made by hand or not at all. Nothing here spawns, maps or invokes: a refusal happens
/// **before the component exists**, which is the property the guest test asserts on by reporting the
/// refusal ahead of the first build step.
pub fn plan(reqs: &Requirements, provisions: &Provisions<'_>) -> Result<Plan, Refusal> {
    if let Some(problem) = reqs.problem() {
        return Err(problem);
    }
    if reqs.caps.len() > MAX_CAPS {
        return Err(Refusal::TooManyCaps {
            asked: reqs.caps.len(),
        });
    }
    if reqs.mapped() > MAX_MAPS {
        return Err(Refusal::TooManyMaps {
            asked: reqs.mapped(),
        });
    }

    let mut out = Plan {
        caps: [(0, 0); MAX_CAPS],
        ncaps: reqs.caps.len(),
        maps: [(0, 0, 0); MAX_MAPS],
        nmaps: reqs.mapped(),
        ndevices: 0,
        pages: reqs.pages,
        handoff: None,
    };

    for (i, need) in reqs.caps.iter().enumerate() {
        let Some(slot) = provisions.slot_for(need.role) else {
            return Err(Refusal::Unprovided { role: need.role });
        };
        out.caps[i] = (slot, need.direction.rights());
    }

    // Two passes over the mapping needs so device registers land at the end of the array whatever
    // order they were declared in. That ordering is what lets `maps_without_devices` be a slice of
    // this same array rather than a second copy a `no_std` caller has nowhere to put.
    let mut n = 0;
    for pass in [PageKind::Shared, PageKind::DeviceRegisters] {
        for need in reqs.maps.iter().filter(|m| m.kind == pass) {
            let Some(slot) = provisions.slot_for(need.role) else {
                return Err(Refusal::Unprovided { role: need.role });
            };
            out.maps[n] = (need.va, slot, need.kind.mode());
            n += 1;
            if pass == PageKind::DeviceRegisters {
                out.ndevices += 1;
            }
        }
        // The handoff page goes in after the shared pages and before any device, so it is part of
        // `maps_without_devices` and a replacement is built holding it. See `Plan::handoff`.
        if pass == PageKind::Shared
            && let Some(h) = reqs.handoff
        {
            let Some(slot) = provisions.slot_for(HANDOFF_ROLE) else {
                return Err(Refusal::Unprovided { role: HANDOFF_ROLE });
            };
            out.maps[n] = (h.va, slot, PageKind::Shared.mode());
            out.handoff = Some(h.va);
            n += 1;
        }
    }

    Ok(out)
}

/// **The capability table slot a component reads one of its own needs in**, derived from its declaration.
///
/// This is the rung the whole crate is built to reach. The slot agreement used to be a pair of
/// constants written twice, once as `pub const SVC: u64 = 0` in the contract and once as the position
/// of an entry in a literal array in the operator, with a comment in each file asking the reader to
/// check. Now the constant *is* the position, computed at compile time, and a role that the manifest
/// does not declare **does not compile**.
///
/// # Panics
///
/// If `role` is not declared. In a `const` item, which is the only way this is meant to be called,
/// that is a compile error rather than a panic.
///
/// ```compile_fail
/// # use component_plan::{CapNeed, Direction, Requirements};
/// const R: Requirements = Requirements {
///     contract: "talker",
///     caps: &[CapNeed { role: "service", direction: Direction::Serve }],
///     maps: &[],
///     pages: 32,
///     depends_on: &[],
///     handoff: None,
/// };
/// // There is no `control` role, so this constant cannot be evaluated.
/// const CONTROL: u64 = component_plan::slot_of(&R, "control");
/// ```
pub const fn slot_of(reqs: &Requirements, role: &str) -> u64 {
    let mut i = 0;
    while i < reqs.caps.len() {
        if str_eq(reqs.caps[i].role, role) {
            return i as u64;
        }
        i += 1;
    }
    panic!("this component declares no capability by that role name")
}

/// Byte equality on two strings, in `const` context. `str`'s own `==` is not a `const fn`, and every
/// check in this crate that has to run at compile time needs one.
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

// ===================================================================================================
// Dependency-aware orchestration (milestone 23's third residual, roadmap's own words: "if component
// B is a client of component A, swapping A means quiesce B, swap, resume").
//
// This is the other half of [`Requirements::depends_on`]'s doc comment, made into a decision rather
// than a fact a supervisor would otherwise have to work out by hand. It answers exactly one
// question: **given the components a supervisor is currently running, which of them must be told
// before a named contract is swapped?** It does not run the swap, does not touch a capability, and
// does not decide *how* a dependent is told (that is a per-contract protocol, `broker`'s
// `BOP_DOWN`/`BOP_UP` today); it only says *who*.
//
// **Direct dependents only, and that is a scope decision recorded rather than an oversight.** A
// dependent that itself absorbs its own dependency's downtime (a queue, in `broker`'s case) is not
// assumed to need telling about *its* dependents in turn, because whether it does is a property of
// that dependent's own mechanism and this crate has no way to know it from two contract names. See
// this crate's `BUGS` for what a transitive version would need and why it is not built.
// ===================================================================================================

/// **How many running components one orchestration query may consider.** The same shape as
/// [`MAX_CAPS`]: a [`Dependents`] is a value on the stack in a `no_std` program with no allocator, so
/// this is a real bound rather than a hint. Eight, matched to [`MAX_CAPS`] because a supervisor
/// managing more live components than it could ever endow one of with capabilities is not a shape
/// this tree has.
pub const MAX_LIVE: usize = 8;

/// **One component a supervisor is currently running**, as far as dependency-aware orchestration
/// needs to know about it: an id the caller chose (a tid, or any label stable for the query) and the
/// contract it is an instance of.
///
/// Not a general process-registry type. It exists to be the input to [`dependents`] and carries
/// nothing that function does not read.
#[derive(Clone, Copy)]
pub struct LiveInstance<'a> {
    /// The caller's own name for this running instance. Opaque to this crate; returned unchanged in
    /// [`Dependents::quiesce_order`].
    pub id: u64,
    /// Which contract this instance implements, and so whose [`Requirements::depends_on`] is
    /// consulted.
    pub reqs: &'a Requirements,
}

/// Why a [`dependents`] query was refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrchestrationRefusal {
    /// More live instances were named than [`MAX_LIVE`] can hold.
    TooManyLive {
        /// How many were asked about.
        asked: usize,
    },
}

/// **The live instances that must be told before a swap, in the order they were found.**
///
/// Not a general ordering claim: nothing here says quiescing one takes longer than another, or that
/// they must be told serially. It is a filter over [`dependents`]'s input, and its shape exists so a
/// supervisor holds the answer as a value rather than re-deriving it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Dependents {
    ids: [u64; MAX_LIVE],
    n: usize,
}

impl Dependents {
    /// The ids to quiesce, in the order [`dependents`] found them (the order `live` was given in).
    /// Empty is a real, common answer: it is what a target with no server-shaped clients returns,
    /// which is every target in this tree's own direct channel.
    pub fn quiesce_order(&self) -> &[u64] {
        &self.ids[..self.n]
    }
}

/// **Which of the running components in `live` must be quiesced before `target_contract` is
/// swapped**, because they declared it in their own [`Requirements::depends_on`].
///
/// An instance of `target_contract` itself is never returned, even if a declaration were malformed
/// enough to list its own contract: a component cannot be its own dependent, and this rules the
/// question out rather than trusting every caller to have avoided asking it.
///
/// # EXAMPLES
///
/// ```
/// use component_plan::{dependents, CapNeed, Direction, LiveInstance, Requirements};
///
/// const BACKEND: Requirements = Requirements {
///     contract: "backend",
///     caps: &[CapNeed { role: "service", direction: Direction::Serve }],
///     maps: &[],
///     pages: 32,
///     depends_on: &[],
///     handoff: None,
/// };
/// const BROKER: Requirements = Requirements {
///     contract: "broker",
///     caps: &[
///         CapNeed { role: "requests", direction: Direction::Serve },
///         CapNeed { role: "backend", direction: Direction::Use },
///     ],
///     maps: &[],
///     pages: 32,
///     depends_on: &["backend"],
///     handoff: None,
/// };
///
/// let live = [
///     LiveInstance { id: 1, reqs: &BACKEND },
///     LiveInstance { id: 2, reqs: &BROKER },
/// ];
///
/// // Swapping the backend means telling the broker first: it forwards to it synchronously.
/// let d = dependents("backend", &live).unwrap();
/// assert_eq!(d.quiesce_order(), &[2]);
///
/// // Swapping the broker itself has no dependents in this registry: nothing here declares
/// // "broker" in its own `depends_on`.
/// let d = dependents("broker", &live).unwrap();
/// assert!(d.quiesce_order().is_empty());
/// ```
pub fn dependents(
    target_contract: &str,
    live: &[LiveInstance],
) -> Result<Dependents, OrchestrationRefusal> {
    if live.len() > MAX_LIVE {
        return Err(OrchestrationRefusal::TooManyLive { asked: live.len() });
    }
    let mut out = Dependents {
        ids: [0; MAX_LIVE],
        n: 0,
    };
    for inst in live {
        if str_eq(inst.reqs.contract, target_contract) {
            continue; // an instance of the target is not its own dependent
        }
        let mut i = 0;
        while i < inst.reqs.depends_on.len() {
            if str_eq(inst.reqs.depends_on[i], target_contract) {
                out.ids[out.n] = inst.id;
                out.n += 1;
                break;
            }
            i += 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVICE: CapNeed = CapNeed {
        role: "service",
        direction: Direction::Serve,
    };
    const REPORT: CapNeed = CapNeed {
        role: "report",
        direction: Direction::Use,
    };
    const WITNESS: MapNeed = MapNeed {
        role: "witness",
        va: 0x0300_0000,
        kind: PageKind::Shared,
    };
    const UART: MapNeed = MapNeed {
        role: "uart",
        va: 0x0310_0000,
        kind: PageKind::DeviceRegisters,
    };

    const CONSOLE: Requirements = Requirements {
        contract: "console",
        caps: &[SERVICE, REPORT],
        maps: &[WITNESS, UART],
        pages: 32,
        depends_on: &[],
        handoff: None,
    };

    fn everything() -> Provisions<'static> {
        Provisions {
            held: &[("service", 5), ("report", 1), ("witness", 9), ("uart", 2)],
        }
    }

    #[test]
    fn a_serve_gets_read_and_a_use_gets_write() {
        let p = plan(&CONSOLE, &everything()).unwrap();
        assert_eq!(p.caps(), &[(5, abi::rights::READ), (1, abi::rights::WRITE)]);
    }

    /// The one that would otherwise be a comment: the plan's cap order is the declaration's order,
    /// which is the component's slot order.
    #[test]
    fn the_declared_order_is_the_slot_order() {
        assert_eq!(slot_of(&CONSOLE, "service"), 0);
        assert_eq!(slot_of(&CONSOLE, "report"), 1);
        let p = plan(&CONSOLE, &everything()).unwrap();
        assert_eq!(p.caps()[slot_of(&CONSOLE, "report") as usize].0, 1);
    }

    #[test]
    fn a_role_the_supervisor_does_not_route_is_refused() {
        let partial = Provisions {
            held: &[("report", 1), ("witness", 9), ("uart", 2)],
        };
        assert_eq!(
            plan(&CONSOLE, &partial),
            Err(Refusal::Unprovided { role: "service" })
        );
    }

    /// A missing *page* refuses exactly as a missing capability does. Worth its own test because the
    /// two are checked in different loops.
    #[test]
    fn a_page_the_supervisor_does_not_route_is_refused_too() {
        let no_device = Provisions {
            held: &[("service", 5), ("report", 1), ("witness", 9)],
        };
        assert_eq!(
            plan(&CONSOLE, &no_device),
            Err(Refusal::Unprovided { role: "uart" })
        );
    }

    /// A supervisor holding more than the component asks for is wired, not refused: it holds what it
    /// holds, and one table per child would be mandatory otherwise.
    #[test]
    fn a_surplus_provision_is_not_an_error() {
        let extra = Provisions {
            held: &[
                ("service", 5),
                ("report", 1),
                ("witness", 9),
                ("uart", 2),
                ("nothing_needs_this", 77),
            ],
        };
        assert!(plan(&CONSOLE, &extra).is_ok());
    }

    /// §41's device deferral, which is the reason [`PageKind::DeviceRegisters`] is a kind at all.
    #[test]
    fn the_device_is_separable_from_the_rest_of_the_mappings() {
        let p = plan(&CONSOLE, &everything()).unwrap();
        assert_eq!(
            p.maps_without_devices(),
            &[(0x0300_0000, 9, abi::address_space::MAP_RW)]
        );
        assert_eq!(p.devices(), &[(0x0310_0000, 2, abi::address_space::MAP_RO)]);
    }

    /// And it is separable whatever order the declaration used, because a plan sorts rather than
    /// trusting the manifest's author to put the device last.
    #[test]
    fn a_device_declared_first_still_sorts_last() {
        const DEVICE_FIRST: Requirements = Requirements {
            contract: "console",
            caps: &[SERVICE],
            maps: &[UART, WITNESS],
            pages: 32,
            depends_on: &[],
            handoff: None,
        };
        let p = plan(&DEVICE_FIRST, &everything()).unwrap();
        assert_eq!(p.devices(), &[(0x0310_0000, 2, abi::address_space::MAP_RO)]);
        assert_eq!(p.maps_without_devices().len(), 1);
    }

    /// A component with no device at all: the empty-device case must not be a panic in the
    /// arithmetic that splits the array.
    #[test]
    fn a_component_with_no_device_has_no_deferred_mapping() {
        const PLAIN: Requirements = Requirements {
            contract: "backend",
            caps: &[SERVICE],
            maps: &[WITNESS],
            pages: 32,
            depends_on: &[],
            handoff: None,
        };
        let p = plan(&PLAIN, &everything()).unwrap();
        assert!(p.devices().is_empty());
        assert_eq!(p.maps_without_devices(), p.maps());
    }

    // ===========================================================================================
    // State handoff (DECISIONS §209).
    // ===========================================================================================

    /// A console that also carries state: the widest shape a handoff can land in, with a device on
    /// the far side of it.
    const STATEFUL: Requirements = Requirements {
        contract: "tally",
        caps: &[SERVICE],
        maps: &[UART, WITNESS],
        pages: 32,
        depends_on: &[],
        handoff: Some(Handoff { va: 0x0320_0000 }),
    };

    fn with_handoff() -> Provisions<'static> {
        Provisions {
            held: &[
                ("service", 5),
                ("report", 1),
                ("witness", 9),
                ("uart", 2),
                (HANDOFF_ROLE, 11),
            ],
        }
    }

    /// **The handoff page is built in, not deferred.** It lands among the shared pages and ahead of
    /// the device, so a replacement built from `maps_without_devices` already holds it, and the
    /// device split the direct swap depends on is exactly what it was.
    #[test]
    fn the_handoff_page_is_mapped_early_and_the_device_still_sorts_last() {
        let p = plan(&STATEFUL, &with_handoff()).unwrap();
        assert_eq!(p.handoff(), Some(0x0320_0000));
        assert_eq!(
            p.maps_without_devices(),
            &[
                (0x0300_0000, 9, abi::address_space::MAP_RW),
                (0x0320_0000, 11, abi::address_space::MAP_RW),
            ]
        );
        assert_eq!(p.devices(), &[(0x0310_0000, 2, abi::address_space::MAP_RO)]);
    }

    /// **The refusal §209 needs and does not spell out.** A supervisor that routes no handoff page
    /// cannot carry state, and wiring the component anyway would be a kill-and-replace that looks
    /// exactly like a successful swap while the state is thrown away. So it is refused before
    /// anything is built, the same way an unrouted capability is.
    #[test]
    fn a_supervisor_that_cannot_carry_state_is_refused_a_component_that_declares_some() {
        let no_state = Provisions {
            held: &[("service", 5), ("witness", 9), ("uart", 2)],
        };
        assert_eq!(
            plan(&STATEFUL, &no_state),
            Err(Refusal::Unprovided { role: HANDOFF_ROLE })
        );
    }

    /// And a component that declares nothing is wired exactly as before, whatever the supervisor
    /// holds: optional is the half of §209 that most of the tree lives in.
    #[test]
    fn a_component_with_no_handoff_ignores_a_supervisor_that_offers_one() {
        let p = plan(&CONSOLE, &with_handoff()).unwrap();
        assert_eq!(p.handoff(), None);
        assert_eq!(p.maps().len(), 2);
    }

    /// The handoff page is one more mapping and obeys both mapping rules: no shared address, and its
    /// role word is not declared twice.
    #[test]
    fn a_handoff_page_is_held_to_the_mapping_rules() {
        const COLLIDES: Requirements = Requirements {
            handoff: Some(Handoff { va: 0x0300_0000 }),
            ..STATEFUL
        };
        assert_eq!(
            COLLIDES.problem(),
            Some(Refusal::OverlappingVa { va: 0x0300_0000 })
        );
        const TWICE: Requirements = Requirements {
            maps: &[MapNeed {
                role: HANDOFF_ROLE,
                va: 0x0400_0000,
                kind: PageKind::Shared,
            }],
            ..STATEFUL
        };
        assert_eq!(
            TWICE.problem(),
            Some(Refusal::DuplicateRole { role: HANDOFF_ROLE })
        );
    }

    /// The bound counts the handoff page: four declared pages plus a handoff is five mappings, and a
    /// plan with room for four must refuse rather than overflow its array.
    #[test]
    fn the_handoff_page_counts_against_the_mapping_bound() {
        const FULL: Requirements = Requirements {
            maps: &[
                MapNeed {
                    role: "a",
                    va: 0x1000,
                    kind: PageKind::Shared,
                },
                MapNeed {
                    role: "b",
                    va: 0x2000,
                    kind: PageKind::Shared,
                },
                MapNeed {
                    role: "c",
                    va: 0x3000,
                    kind: PageKind::Shared,
                },
                MapNeed {
                    role: "d",
                    va: 0x4000,
                    kind: PageKind::Shared,
                },
            ],
            ..STATEFUL
        };
        assert_eq!(
            plan(&FULL, &with_handoff()),
            Err(Refusal::TooManyMaps { asked: 5 })
        );
    }

    #[test]
    fn a_role_declared_twice_is_malformed() {
        const TWICE: Requirements = Requirements {
            contract: "confused",
            caps: &[SERVICE, SERVICE],
            maps: &[],
            pages: 32,
            depends_on: &[],
            handoff: None,
        };
        assert_eq!(
            TWICE.problem(),
            Some(Refusal::DuplicateRole { role: "service" })
        );
        assert_eq!(plan(&TWICE, &everything()).unwrap_err().code(), 2);
    }

    #[test]
    fn two_pages_at_one_address_are_malformed() {
        const COLLIDE: Requirements = Requirements {
            contract: "confused",
            caps: &[],
            maps: &[
                WITNESS,
                MapNeed {
                    role: "other",
                    va: WITNESS.va,
                    kind: PageKind::Shared,
                },
            ],
            pages: 32,
            depends_on: &[],
            handoff: None,
        };
        assert_eq!(
            COLLIDE.problem(),
            Some(Refusal::OverlappingVa { va: 0x0300_0000 })
        );
    }

    #[test]
    fn a_component_nothing_pays_for_is_malformed() {
        const FREE: Requirements = Requirements {
            contract: "confused",
            caps: &[],
            maps: &[],
            pages: 0,
            depends_on: &[],
            handoff: None,
        };
        assert_eq!(FREE.problem(), Some(Refusal::NoPages));
    }

    /// Every refusal has a distinct wire code, because the guest reports the code and a collision
    /// would make two different failures indistinguishable in a test.
    #[test]
    fn the_refusal_codes_are_distinct_and_never_zero() {
        let all = [
            Refusal::Unprovided { role: "x" },
            Refusal::DuplicateRole { role: "x" },
            Refusal::OverlappingVa { va: 0 },
            Refusal::TooManyCaps { asked: 0 },
            Refusal::TooManyMaps { asked: 0 },
            Refusal::NoPages,
        ];
        for (i, a) in all.iter().enumerate() {
            assert_ne!(a.code(), 0, "zero would read as \"no refusal\"");
            assert!(!a.message().is_empty());
            for b in &all[i + 1..] {
                assert_ne!(a.code(), b.code());
            }
        }
    }

    #[test]
    fn the_well_formed_declaration_is_the_common_case() {
        assert_eq!(CONSOLE.problem(), None);
    }

    // ===========================================================================================
    // Dependency-aware orchestration.
    // ===========================================================================================

    const BACKEND: Requirements = Requirements {
        contract: "backend",
        caps: &[SERVICE],
        maps: &[],
        pages: 32,
        depends_on: &[],
        handoff: None,
    };
    const BROKER: Requirements = Requirements {
        contract: "broker",
        caps: &[
            CapNeed {
                role: "requests",
                direction: Direction::Serve,
            },
            CapNeed {
                role: "backend",
                direction: Direction::Use,
            },
        ],
        maps: &[],
        pages: 32,
        depends_on: &["backend"],
        handoff: None,
    };
    const CLIENT: Requirements = Requirements {
        contract: "client",
        caps: &[REPORT],
        maps: &[],
        pages: 32,
        depends_on: &[],
        handoff: None,
    };

    /// The one case this residual exists for, in its smallest true form: a component that forwards
    /// synchronously to another must be told before that other is swapped.
    #[test]
    fn a_component_that_calls_through_to_a_swap_target_is_a_dependent() {
        let live = [
            LiveInstance {
                id: 1,
                reqs: &BACKEND,
            },
            LiveInstance {
                id: 2,
                reqs: &BROKER,
            },
        ];
        let d = dependents("backend", &live).unwrap();
        assert_eq!(d.quiesce_order(), &[2]);
    }

    /// A pure consumer never appears, because it has nothing declared in `depends_on`: §41 already
    /// proved a `CALL` to an absent server degrades for free (the kernel's own sender queue), so a
    /// component with no `Serve` need of its own never needs telling.
    #[test]
    fn a_pure_consumer_is_never_a_dependent() {
        let live = [
            LiveInstance {
                id: 1,
                reqs: &BACKEND,
            },
            LiveInstance {
                id: 3,
                reqs: &CLIENT,
            },
        ];
        let d = dependents("backend", &live).unwrap();
        assert!(
            d.quiesce_order().is_empty(),
            "CLIENT declares no depends_on, so it must never be returned",
        );
    }

    /// Swapping a contract that nothing depends on is the common case in this tree's own direct
    /// channel, and it must be a real empty answer rather than a refusal.
    #[test]
    fn a_target_with_no_dependents_returns_an_empty_order() {
        let live = [LiveInstance {
            id: 1,
            reqs: &BACKEND,
        }];
        let d = dependents("backend", &live).unwrap();
        assert!(d.quiesce_order().is_empty());
    }

    /// An instance of the target contract is never its own dependent, even if asked about directly:
    /// a component cannot need warning about its own absence.
    #[test]
    fn an_instance_of_the_target_itself_is_excluded() {
        let live = [
            LiveInstance {
                id: 1,
                reqs: &BACKEND,
            },
            LiveInstance {
                id: 2,
                reqs: &BROKER,
            },
        ];
        let d = dependents("broker", &live).unwrap();
        assert!(
            d.quiesce_order().is_empty(),
            "nothing in this registry declares broker as a dependency, and the broker instance \
             itself must not be returned even though it exists",
        );
    }

    /// More live instances than `MAX_LIVE` is refused rather than silently truncated: a supervisor
    /// that got a short answer back would quiesce fewer components than it needed to.
    #[test]
    fn too_many_live_instances_is_refused() {
        let entries: [LiveInstance; MAX_LIVE + 1] = [LiveInstance {
            id: 0,
            reqs: &BACKEND,
        }; MAX_LIVE + 1];
        assert_eq!(
            dependents("backend", &entries),
            Err(OrchestrationRefusal::TooManyLive {
                asked: MAX_LIVE + 1
            })
        );
    }

    /// Order is preserved from the caller's own `live` slice, not sorted or otherwise rearranged:
    /// [`dependents`] is a filter, and a supervisor that wants a particular quiesce order gets it by
    /// choosing the order it lists its live instances in.
    #[test]
    fn the_order_returned_is_the_order_given() {
        const OTHER_BROKER: Requirements = Requirements {
            contract: "other_broker",
            caps: &[],
            maps: &[],
            pages: 32,
            depends_on: &["backend"],
            handoff: None,
        };
        let live = [
            LiveInstance {
                id: 20,
                reqs: &OTHER_BROKER,
            },
            LiveInstance {
                id: 10,
                reqs: &BROKER,
            },
        ];
        let d = dependents("backend", &live).unwrap();
        assert_eq!(d.quiesce_order(), &[20, 10]);
    }
}

/// Machine-checked properties of the wiring. The host tests above pin the cases a reader cares
/// about; these rule out shapes a case-by-case test cannot, and the symbolism is in the **routing
/// table** rather than in the declaration.
///
/// That split is forced by the type and is worth understanding rather than working around.
/// [`Requirements`] holds `&'static` slices, so a declaration is a compile-time constant by
/// construction: it is what makes [`Requirements::problem`] and [`slot_of`] evaluable in a `const`
/// item, and it means **no manifest can be synthesised at run time out of bytes**, which is exactly
/// the property the wire-format entry in this crate's `BUGS` section would have to give up. So the
/// declarations below are a bounded enumeration of `const` values (both directions, all four
/// arrangements of the two page kinds), walked by a **concrete** loop, and what is symbolic is what a
/// supervisor actually varies: slot numbers, and which role names are routed at all.
///
/// The concrete loop is not a stylistic choice. Selecting a declaration with `SHAPES[kani::any()]`
/// makes the slice CBMC reads a symbolic pointer, and every slice access downstream of it becomes
/// symbolic too: that formulation put the solver past **seventy thousand** verification conditions
/// for a fact about copying integers, and did not finish in ninety seconds of symbolic execution.
/// Walking the universe concretely and leaving the *routing* symbolic proves the same statement
/// (both are "for every declaration of this width") and is a viable gate. Recorded because the
/// difference is invisible from the assertion.
///
/// See notes/component-manifest.md and notes/verification.md.
#[cfg(kani)]
mod proofs {
    use super::*;

    const SERVE_A: CapNeed = CapNeed {
        role: "a",
        direction: Direction::Serve,
    };
    const USE_A: CapNeed = CapNeed {
        role: "a",
        direction: Direction::Use,
    };
    const SERVE_B: CapNeed = CapNeed {
        role: "b",
        direction: Direction::Serve,
    };
    const USE_B: CapNeed = CapNeed {
        role: "b",
        direction: Direction::Use,
    };

    /// Every arrangement of two capability needs over the two directions. Four, and that is the whole
    /// space for two roles, so "for every declaration of this width" is exact rather than sampled.
    const SHAPES: [Requirements; 4] = [
        Requirements {
            contract: "c",
            caps: &[SERVE_A, SERVE_B],
            maps: &[],
            pages: 1,
            depends_on: &[],
            handoff: None,
        },
        Requirements {
            contract: "c",
            caps: &[SERVE_A, USE_B],
            maps: &[],
            pages: 1,
            depends_on: &[],
            handoff: None,
        },
        Requirements {
            contract: "c",
            caps: &[USE_A, SERVE_B],
            maps: &[],
            pages: 1,
            depends_on: &[],
            handoff: None,
        },
        Requirements {
            contract: "c",
            caps: &[USE_A, USE_B],
            maps: &[],
            pages: 1,
            depends_on: &[],
            handoff: None,
        },
    ];

    /// **A slot a supervisor could route, bounded to the capability table that exists.**
    ///
    /// `abi::CAPABILITY_TABLE_SLOTS` is sixteen, so a slot number outside `0..16` is not a routing a supervisor
    /// could make: the kernel refuses it. Bounding it here is not a weakening of the property, which
    /// is about *which* slot a need is wired to rather than how large the number is, and it is what
    /// makes these harnesses a viable gate: unbounded `u64` slots put the solver past seventy
    /// thousand verification conditions for a fact about copying two integers through.
    fn any_slot() -> u64 {
        let s: u64 = kani::any();
        kani::assume(s < abi::CAPABILITY_TABLE_SLOTS);
        s
    }

    /// **No routing table ever gives a component a right its declaration did not ask for.**
    ///
    /// The security property this crate exists to make unrepresentable. For every declaration of two
    /// needs and **every pair of slot numbers a supervisor could route**, the rights word at slot `i`
    /// is exactly `caps[i].direction.rights()`. So nothing a supervisor does can turn a `Use` into a
    /// `READ` and let a client park itself in `RECV_CAP` on the rendezvous it is a client of (§41), and
    /// nothing ever carries `GRANT`: a component that could pass its own authority on is not
    /// confined, and no declaration can ask for one.
    /// **The unwind bound is a termination proof, not a convenience.** Ten, against loops whose real
    /// bounds are two capability needs, two page needs, two routed names and a one-byte role name.
    /// It is needed at all because `SHAPES[i]` with a symbolic `i` makes the slice CBMC reads a
    /// symbolic pointer, so the lengths it walks are symbolic to the solver even though only four
    /// concrete declarations exist. Without a bound `str_eq` unwinds forever.
    /// Falsification: replayable `crates/component_plan/falsifications/proofs.a_plan_never_grants_a_right_the_declaration_did_not_ask_for.patch`
    #[kani::proof]
    #[kani::unwind(10)]
    fn a_plan_never_grants_a_right_the_declaration_did_not_ask_for() {
        let held: [(&str, u64); 2] = [("a", any_slot()), ("b", any_slot())];
        for reqs in &SHAPES {
            let p = plan(reqs, &Provisions { held: &held }).expect("both roles are routed");
            assert!(p.caps().len() == 2);
            let mut i = 0;
            while i < 2 {
                // **The mapping written out, not `rights()` compared with itself** (milestone
                // 211, and milestone 202 found this shape here first). `plan` fills the rights
                // word from `direction.rights()`, so an equality against that same call is
                // satisfied by any consistently wrong `rights()`: one that returned
                // `READ | GRANT` for both directions passes it. The two assertions below it
                // were already independent, and they are what caught the defect; this makes the
                // first one independent too, so the harness states the whole claim rather than
                // two thirds of it.
                let expected = match reqs.caps[i].direction {
                    Direction::Serve => abi::rights::READ,
                    Direction::Use => abi::rights::WRITE,
                };
                assert!(
                    p.caps()[i].1 == expected,
                    "a plan granted a right the declaration did not ask for",
                );
                // **The two assertions that used to follow are gone, and which one they were is
                // the point** (milestone 307). They were `== READ || == WRITE` and
                // `& GRANT == 0`, and once the line above was written out in literals rather than
                // through `direction.rights()` (milestone 211), both became implied by it:
                // `READ = 1<<0`, `WRITE = 1<<1`, `GRANT = 1<<2`, so an equality against one of
                // the first two settles the other two. The `& GRANT == 0` line is the one
                // notes/confinement-claims.md credited as the only thing catching the original
                // defect, and 211's fix turned the rescue assertion into the decoration. That
                // inversion is worth more than the line was.

                assert!(p.caps()[i].0 == held[i].1);
                i += 1;
            }
        }
    }

    /// **A missing route refuses rather than falling through to a slot.**
    ///
    /// The failure this rules out is the quiet one. A lookup that fell through to zero would endow a
    /// child with whatever the supervisor keeps in slot 0, which in every program in this tree is its
    /// construction budget. Proved over every arrangement of the two names being routed or not, and
    /// over arbitrary slots.
    /// Falsification: unfalsified
    #[kani::proof]
    #[kani::unwind(10)]
    fn a_missing_route_refuses_rather_than_falling_through_to_a_slot() {
        // Each entry is present under its own name, or under a name nothing declares.
        let n0: &str = if kani::any() { "a" } else { "z" };
        let n1: &str = if kani::any() { "b" } else { "z" };
        let (s0, s1) = (any_slot(), any_slot());
        let held: [(&str, u64); 2] = [(n0, s0), (n1, s1)];
        for reqs in &SHAPES {
            match plan(reqs, &Provisions { held: &held }) {
                Ok(p) => {
                    // Wired at all only if both names were routed, and then to those slots and no
                    // others.
                    assert!(n0 == "a" && n1 == "b");
                    assert!(p.caps()[0].0 == s0 && p.caps()[1].0 == s1);
                }
                Err(r) => {
                    assert!(n0 != "a" || n1 != "b");
                    assert!(r.code() == Refusal::Unprovided { role: "a" }.code());
                }
            }
        }
    }

    const SHARED_P: MapNeed = MapNeed {
        role: "p",
        va: 0x1000,
        kind: PageKind::Shared,
    };
    const DEVICE_P: MapNeed = MapNeed {
        role: "p",
        va: 0x1000,
        kind: PageKind::DeviceRegisters,
    };
    const SHARED_Q: MapNeed = MapNeed {
        role: "q",
        va: 0x2000,
        kind: PageKind::Shared,
    };
    const DEVICE_Q: MapNeed = MapNeed {
        role: "q",
        va: 0x2000,
        kind: PageKind::DeviceRegisters,
    };

    /// Every arrangement of two page needs over the two kinds, which is the whole space for two
    /// pages: neither a device, either one, or both.
    const PAGE_SHAPES: [Requirements; 4] = [
        Requirements {
            contract: "c",
            caps: &[],
            maps: &[SHARED_P, SHARED_Q],
            pages: 1,
            depends_on: &[],
            handoff: None,
        },
        Requirements {
            contract: "c",
            caps: &[],
            maps: &[DEVICE_P, SHARED_Q],
            pages: 1,
            depends_on: &[],
            handoff: None,
        },
        Requirements {
            contract: "c",
            caps: &[],
            maps: &[SHARED_P, DEVICE_Q],
            pages: 1,
            depends_on: &[],
            handoff: None,
        },
        Requirements {
            contract: "c",
            caps: &[],
            maps: &[DEVICE_P, DEVICE_Q],
            pages: 1,
            depends_on: &[],
            handoff: None,
        },
    ];

    /// **The device split is a partition, for every declaration.**
    ///
    /// This is the arithmetic §41's four-syscall down window rests on. The operator endows a
    /// replacement with [`Plan::maps_without_devices`] and installs [`Plan::devices`] after the
    /// revoke, so an entry falling in both would be mapped twice and one falling in neither would
    /// leave a component reading an unmapped page. Proved for every arrangement of kinds, including
    /// a device declared first, which is what the sort exists for.
    /// Falsification: replayable `crates/component_plan/falsifications/proofs.the_device_split_partitions_the_mappings.patch`
    #[kani::proof]
    #[kani::unwind(10)]
    fn the_device_split_partitions_the_mappings() {
        let held: [(&str, u64); 2] = [("p", any_slot()), ("q", any_slot())];
        for reqs in &PAGE_SHAPES {
            let p = plan(reqs, &Provisions { held: &held }).expect("both pages are routed");
            let (early, late) = (p.maps_without_devices(), p.devices());
            assert!(p.maps().len() == 2);
            assert!(early.len() + late.len() == p.maps().len());

            let mut declared_devices = 0;
            let mut k = 0;
            while k < reqs.maps.len() {
                if reqs.maps[k].kind == PageKind::DeviceRegisters {
                    declared_devices += 1;
                }
                k += 1;
            }
            assert!(late.len() == declared_devices);
            // The late half is all devices and the early half is all ordinary shared pages, so the
            // operator's two-step endowment cannot install one of them at the wrong moment.
            // The two mode words spelled out rather than fetched from `kind.mode()`, which is
            // the call `plan` itself makes to fill them (milestone 211): a `mode()` returning
            // one constant for both kinds would satisfy an assertion phrased through it while
            // the operator mapped a device register page writable.
            for m in late {
                assert!(m.2 == abi::address_space::MAP_RO);
            }
            for m in early {
                assert!(m.2 == abi::address_space::MAP_RW);
            }
            const { assert!(abi::address_space::MAP_RO != abi::address_space::MAP_RW) };
        }
    }

    // ===========================================================================================
    // Dependency-aware orchestration.
    // ===========================================================================================

    const TARGET_SELF: Requirements = Requirements {
        contract: "a",
        caps: &[],
        maps: &[],
        pages: 1,
        depends_on: &[],
        handoff: None,
    };
    const OTHER_NODEP: Requirements = Requirements {
        contract: "other",
        caps: &[],
        maps: &[],
        pages: 1,
        depends_on: &[],
        handoff: None,
    };
    const OTHER_DEP: Requirements = Requirements {
        contract: "other",
        caps: &[],
        maps: &[],
        pages: 1,
        depends_on: &["a"],
        handoff: None,
    };

    /// Every arrangement a two-instance registry can be in, with respect to one target contract
    /// `"a"`: an instance of the target itself, an unrelated instance that does not depend on it,
    /// and an unrelated instance that does. Three, so "for every registry of this width" is exact.
    const REG_SHAPES: [Requirements; 3] = [TARGET_SELF, OTHER_NODEP, OTHER_DEP];

    /// Membership, decided by `core`'s string equality rather than by this crate's `str_eq`.
    /// The distinction is the whole point of the helper: see the note in
    /// `dependents_finds_exactly_the_non_target_instances_that_declared_it`.
    fn declares_by_core_eq(depends_on: &[&str], name: &str) -> bool {
        let mut i = 0;
        while i < depends_on.len() {
            if depends_on[i] == name {
                return true;
            }
            i += 1;
        }
        false
    }

    /// **`dependents` returns exactly the non-target instances that declared the target, in the
    /// order they were given, and nothing else.**
    ///
    /// This is the property the whole mechanism rests on: a supervisor that trusted this function to
    /// find every component with a synchronous dependency on the one it is about to swap must not be
    /// told fewer than exist (a component left unwarned mid-swap) or more (a component quiesced for
    /// no reason). Proved over every arrangement two live instances can be in, at every id a
    /// supervisor could have chosen for them.
    /// Falsification: replayable `crates/component_plan/falsifications/proofs.dependents_finds_exactly_the_non_target_instances_that_declared_it.patch`
    #[kani::proof]
    #[kani::unwind(10)]
    fn dependents_finds_exactly_the_non_target_instances_that_declared_it() {
        let ids: [u64; 2] = [any_slot(), any_slot()];
        for r0 in &REG_SHAPES {
            for r1 in &REG_SHAPES {
                let live = [
                    LiveInstance {
                        id: ids[0],
                        reqs: r0,
                    },
                    LiveInstance {
                        id: ids[1],
                        reqs: r1,
                    },
                ];
                let d = dependents("a", &live).expect("two instances is under MAX_LIVE");
                // **The expectation is computed with Rust's own `==`, never with `str_eq`**
                // (milestone 211). `dependents` decides membership by calling `str_eq`, so an
                // expectation that called it too would prove the two agree rather than that
                // either is right: a `str_eq` stuck at `true` would make `dependents` return
                // every instance and make this harness expect every instance.
                let expect0 = r0.contract != "a" && declares_by_core_eq(r0.depends_on, "a");
                let expect1 = r1.contract != "a" && declares_by_core_eq(r1.depends_on, "a");
                let out = d.quiesce_order();

                let mut want_len = 0;
                if expect0 {
                    want_len += 1;
                }
                if expect1 {
                    want_len += 1;
                }
                assert!(out.len() == want_len);

                // Order is the order `live` was given in: index 0's id, if present, precedes
                // index 1's.
                let mut idx = 0;
                if expect0 {
                    assert!(out[idx] == ids[0]);
                    idx += 1;
                }
                if expect1 {
                    assert!(out[idx] == ids[1]);
                }
            }
        }
    }

    /// **A registry past `MAX_LIVE` is refused rather than silently truncated**, for every length
    /// one past the bound could conceivably be constructed at (the proof only needs one, since the
    /// check is a single `>` against a fixed constant, but the point of proving it at all is that a
    /// truncated answer here is the dangerous failure: a supervisor that got fewer dependents back
    /// than exist would swap out from under an unwarned client).
    /// Falsification: unfalsified
    #[kani::proof]
    fn too_many_live_instances_never_silently_truncates() {
        let entries: [LiveInstance; MAX_LIVE + 1] = [LiveInstance {
            id: 0,
            reqs: &OTHER_DEP,
        }; MAX_LIVE + 1];
        assert!(matches!(
            dependents("a", &entries),
            Err(OrchestrationRefusal::TooManyLive { asked }) if asked == MAX_LIVE + 1
        ));
    }
}
