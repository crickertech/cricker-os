//! **A confined process that issues random syscalls, one at a time, under a conductor** (milestone
//! 779 (fuzz the surface a confined process can reach), provisional).
//!
//! `system_tests/src/user/confined_fuzzer_tests.rs` spawns three of these with the endowment
//! `confined_fuzz_protocol` lays out, and between every two calls it checks that none of them holds
//! a capability, or maps a page, that it was never granted. This program is the generator half: it
//! waits on [`GO`], makes one call drawn from its seed, writes what it did to the report page, and
//! waits again. It never interprets an answer, so a wrong answer cannot steer it anywhere a right
//! one would not; the oracle is entirely the conductor's.
//!
//! `arg0` is the seed and `arg1` the role (`confined_fuzz_protocol::ROLE_*`). The seed alone
//! decides every call, so a run replays from it.
//!
//! **Why an EL0 program and not milestone 752 (a seeded syscall driver with a shadow model)'s kernel threads.** 752's driver enters through
//! `syscall::invoke` from kernel threads, which skips the trap entry, the register marshaling, and
//! any check that depends on the caller having a user address space. This process is spawned with
//! `user::run` like any confined program, holds only its grants, and reaches the kernel through the
//! same `svc`/`ecall`/`syscall` every program does.
//!
//! # What the generator reaches
//!
//! `SYS_INVOKE` on any slot but [`GO`] with any method and any words, biased toward the endowment and
//! the methods objects have; `SYS_CAP_DELETE`; `SYS_YIELD`; and unknown syscall numbers. Words are
//! drawn from small integers, slot numbers, rights masks, page-aligned addresses in the playground,
//! on the report page and in the image and stack, the ABI's tag values, and raw random words.
//!
//! # BUGS
//!
//! - **[`GO`] is never fuzzed.** It is the one capability the conductor needs, so the generator
//!   never names it. It holds `READ` alone, which leaves `RECEIVE` and `RECEIVE_CAP` on it unexercised.
//! - **No thread or timer is ever made.** A `MemoryRegion::RETYPE_OBJ` asking for a TCB or a timer is
//!   rewritten to ask for a rendezvous, because a second thread running at a random entry, or a timer
//!   firing on the clock, would make a seed's run depend on timing, and the seed is the reproducer.
//!   Both object types, and every method on them, are therefore out of reach.
//! - **`SYS_EXIT` is never drawn.** It would end the run, which teaches the oracle nothing.
//! - **It never touches the memory it maps.** A load or store into a mapped page is not a syscall,
//!   and the page oracle reads the page tables directly instead.
//!
//! Name: provisional (milestone 779 (fuzz the surface a confined process can reach)'s lane, 2026-10-06 UTC).

#![no_std]
// Program entry points, not the crates/ library surface milestone 68 (code-quality gates)'s ratchet tracks
// (DECISIONS §107 (`missing_docs` moves to `workspace.lints.rust`)): each `[[bin]]` is its own crate root with one `_start`.
#![allow(missing_docs)]
#![no_main]

use abi::{SYS_EXIT, SYS_INVOKE, SYS_YIELD};
use confined_fuzz_protocol::{
    ENDOWED, GO, GO_EXIT, NO_CAPABILITY, PLAYGROUND_PAGES, PLAYGROUND_VA, REPORT_VA,
    SYS_CAPABILITY_DELETE, report,
};
use user_mode_runtime::{exit, raw_syscall, receive};

const SLOTS: u64 = abi::CAPABILITY_TABLE_SLOTS;
const PAGE: u64 = address_space_map::PAGE;

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

    /// A slot: mostly the endowment and the slots just above it, where delegations land; sometimes
    /// anywhere in the table; sometimes past it. Never [`GO`].
    fn slot(&mut self) -> u64 {
        let s = match self.below(10) {
            0..=4 => self.below(ENDOWED),
            5..=7 => self.below(ENDOWED + 8),
            8 => self.below(SLOTS),
            _ => match self.below(3) {
                0 => SLOTS,
                1 => u64::MAX,
                _ => self.next(),
            },
        };
        if s == GO { GO + 1 } else { s }
    }

    /// A virtual address worth trying: mostly a playground page, sometimes one the process already
    /// uses (the report page, its image, its stack), sometimes one no user mapping may have.
    fn address(&mut self) -> u64 {
        match self.below(10) {
            0..=5 => PLAYGROUND_VA + self.below(PLAYGROUND_PAGES) * PAGE,
            6 => REPORT_VA,
            7 => address_space_map::IMAGE_BASE + self.below(4) * PAGE,
            8 => address_space_map::STACK_TOP_PAGE,
            _ => match self.below(3) {
                0 => PLAYGROUND_VA + self.below(PAGE), // misaligned
                1 => 0xFFFF_0000_0000_0000 | (self.next() & 0xFFFF_F000), // the kernel half
                _ => self.next(),
            },
        }
    }

    /// A word for an argument register.
    fn word(&mut self) -> u64 {
        match self.below(12) {
            0 | 1 => self.below(8),
            2 | 3 => self.slot(),
            4 => self.below(16), // a rights mask, ENUMERATE included
            5 | 6 => self.address(),
            7 => 1 + self.below(4), // a page count
            8 => NO_CAPABILITY,
            9 => match self.below(3) {
                0 => abi::rendezvous::REPLY_DELIVERED,
                1 => abi::notification::BOUND,
                _ => (abi::Error::NoSuchSlot as i64) as u64,
            },
            10 => self.below(1 << 16),
            _ => self.next(),
        }
    }

    /// A method number: mostly one some object answers, sometimes not.
    fn method(&mut self) -> u64 {
        if self.chance(88) {
            self.below(8)
        } else if self.chance(50) {
            self.below(32)
        } else {
            self.next()
        }
    }
}

/// One call: a syscall number and its six words.
fn pick(rng: &mut Rng) -> (u64, [u64; 6]) {
    let (nr, mut words) = draw(rng);
    // `MemoryRegion::RETYPE_OBJ`'s first word is the object type. A TCB or a timer would run on its
    // own clock and break the seed's determinism (see the module's `BUGS`), so either becomes a
    // rendezvous. Applied whatever the slot holds, which nudges a `SEND_CAP`'s capability argument
    // off 3 and 5 too; that is the price of not knowing the slot's type.
    if nr == SYS_INVOKE
        && words[1] == abi::memory_region::RETYPE_OBJ
        && (words[2] == abi::objtype::THREAD_CONTROL_BLOCK || words[2] == abi::objtype::TIMER)
    {
        words[2] = abi::objtype::RENDEZVOUS;
    }
    (nr, words)
}

fn draw(rng: &mut Rng) -> (u64, [u64; 6]) {
    let roll = rng.below(100);
    if roll < 88 {
        let slot = rng.slot();
        let method = rng.method();
        (
            SYS_INVOKE,
            [slot, method, rng.word(), rng.word(), rng.word(), 0],
        )
    } else if roll < 94 {
        (SYS_CAPABILITY_DELETE, [rng.slot(), 0, 0, 0, 0, 0])
    } else if roll < 96 {
        (SYS_YIELD, [0; 6])
    } else {
        // A number no syscall answers, or one a little past the last real one.
        let nr = match rng.below(3) {
            0 => 4 + rng.below(12),
            1 => rng.below(1 << 16),
            _ => rng.next(),
        };
        let nr = if nr == SYS_EXIT { SYS_YIELD } else { nr };
        let mut words = [0; 6];
        for w in &mut words {
            *w = rng.word();
        }
        if words[0] == GO {
            words[0] = GO + 1;
        }
        (nr, words)
    }
}

fn report_word(i: usize, v: u64) {
    // SAFETY: the conductor maps one read-write page at REPORT_VA before this process runs, and
    // `report::WORDS` fits in it. Volatile so the conductor, reading the frame from the kernel,
    // sees every store in order when this thread is parked on GO.
    unsafe { core::ptr::write_volatile((REPORT_VA as *mut u64).add(i), v) }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(seed: u64, role: u64, _arg2: u64) -> ! {
    let mut rng = Rng(splitmix(
        seed ^ role.wrapping_add(1).wrapping_mul(0xA076_1D64_78BD_642F),
    ));
    let mut steps = 0u64;
    loop {
        report_word(report::STEPS, steps);
        let (word, _, _) = receive(GO);
        if word == GO_EXIT {
            exit()
        }
        let (nr, words) = pick(&mut rng);
        report_word(report::NUMBER, nr);
        for (i, &w) in words.iter().enumerate() {
            report_word(report::ARGS + i, w);
        }
        // SAFETY: the point of the program. The kernel validates every word before acting on it;
        // `pick` never draws SYS_EXIT and never names GO.
        let back = unsafe { raw_syscall(nr, words) };
        for (i, &w) in back.iter().enumerate() {
            report_word(report::RETURNS + i, w);
        }
        steps += 1;
    }
}

user_mode_runtime::panic_handler!();
