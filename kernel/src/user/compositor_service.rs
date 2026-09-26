use compositor::SCENE;

use super::*;
use crate::cap::{Rights, memory_region_cap, page_frame_run_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// **The budget the compositor, and a capture client, draw their own screen mapping's page tables
/// from** (DECISIONS §102, milestone 142). Same reasoning as `display_service::MAP_BUDGET_PAGES`,
/// including that lane's caveat: at the 1280x720 scanout this was widened for, the screen alone
/// spanned more than one 2 MiB window; at the 924x344 scanout the milestone was retargeted to on
/// 2026-08-27 ([`SCREEN_PAGE_FRAMES`], 311 pages) it is back under one window, the same shape a
/// smaller screen always had, and this constant is kept at the wider value rather than re-tuned
/// down.
const MAP_BUDGET_PAGES: u64 = 24;

// The compositor's address space. Must match components/src/compositor.rs. `SCREEN_VA` is not here: the
// screen is a `PageFrame` capability now (§102), and the compositor picks its own VA for it (like
// `painter`/`display_terminal` already do for rung one's surface), so the kernel wiring has no
// reason to know the address. `WLIST_VA`/`RING_VA`/`CLIENT_BASE` moved clear of the address range
// the grown screen ([`SCREEN_PAGE_FRAMES`] page frames, up to 4 MiB from `SCREEN_VA`, sized for the
// largest scanout this milestone has used rather than today's 924x344/311-frame one) now claims in
// the compositor's own space; see `components/src/compositor.rs`'s matching comment for the arithmetic.
const WLIST_VA: u64 = address_space_map::pair_page(0x0000_0000_0c00_0000);
const RING_VA: u64 = address_space_map::pair_page(0x0000_0000_0c01_0000);
const CLIENT_BASE: u64 = address_space_map::pair_page(0x0000_0000_0e00_0000);
const CLIENT_STRIDE: u64 = 0x0000_0000_0010_0000;

// A client's address space. Must match fixtures/src/window.rs. The same in every client, on purpose.
// `C_SCREEN_VA` is likewise not here, for `SCREEN_VA`'s own reason above. `C_WLIST_VA` moved for
// the same reason `compositor.rs`'s own `WLIST_VA` did: it used to sit just past the screen's old,
// tiny span and is now well clear of the grown one (`window.rs`'s own comment has the arithmetic).
const CTL_VA: u64 = address_space_map::pair_page(0x0000_0000_0060_0000);
const SURFACE_VA: u64 = address_space_map::pair_page(0x0000_0000_0061_0000);
const C_WLIST_VA: u64 = address_space_map::pair_page(0x0000_0000_0c00_0000);

// Client roles. Must match fixtures/src/window.rs.
pub const ROLE_INPUT: u64 = 1 << 0;
pub const ROLE_PROBE_INPUT: u64 = 1 << 1;
pub const ROLE_PROBE_NEIGHBOUR: u64 = 1 << 2;
pub const ROLE_PROBE_SCREEN: u64 = 1 << 3;
pub const ROLE_CAPTURE: u64 = 1 << 4;
pub const ROLE_VICTIM: u64 = 1 << 5;
pub const ROLE_SMALL_DAMAGE: u64 = 1 << 6;

/// The screen's frames, in the scanout's own geometry. The same run of frames rung one's driver
/// scans out, because the screen *is* rung one's surface.
const SCREEN_PAGE_FRAMES: core::num::NonZeroU64 =
    crate::cap::page_frame_run_len(graphics_protocol::SURFACE_PAGE_FRAMES as u64);

/// What the kernel keeps after wiring a scene: the endpoints it can ring or listen on, and the
/// physical addresses it needs to be an independent witness of what the processes did.
///
/// The physical addresses are the point of this struct. The kernel allocated these frames, so it
/// can read them through the direct map without asking any process anything, which is what lets a
/// test check a client's surface against a value it computed itself rather than against a number
/// the client reported.
pub struct Wiring {
    /// Where clients (and the input source) ring. The kernel holds WRITE so a test can play the
    /// input driver.
    pub doorbell: RendezvousId,
    /// The compositor's report endpoint.
    pub report: RendezvousId,
    /// The screen's frames.
    pub screen: u64,
    /// The window-list page the compositor publishes.
    pub wlist: u64,
    /// The input ring page, shared with the input source and nobody else.
    pub ring: u64,
    /// Each client's frames: its control page, then its surface.
    pub client: [u64; compositor::MAX_WINDOWS],
    /// Each client's report endpoint. One per client, so the kernel knows who is speaking: the
    /// kernel is the spawner and may hold per-client channels, which is exactly the identity the
    /// compositor deliberately does not have.
    pub client_report: [RendezvousId; compositor::MAX_WINDOWS],
    /// Each focusable client's input endpoint (the compositor holds WRITE, the client READ).
    pub input: [RendezvousId; compositor::MAX_WINDOWS],
    pub n: usize,
    pub focusable: usize,
    image: &'static [u8],
    /// The display terminal's image (milestone 29's text increment), so a scene can be built out
    /// of window clients, terminals, or both. A terminal is a window client with a different
    /// program inside it and exactly the same authority.
    term_image: &'static [u8],
    ring_tail: u32,
}

/// **Wire the scene and start the compositor.** `display` is an endpoint speaking the rung-one
/// display contract (`gpu_driver`, or a kernel stand-in for the tests that do not need a device), and
/// `screen` the frames it scans out.
///
/// Returns once the compositor is spawned; the caller should wait for `status::COMP_UP` on
/// [`Wiring::report`] before spawning clients, which is also the reason a client's first act is a
/// content-free `HELLO`: either order works.
pub fn start(n: usize, focusable: usize, display: RendezvousId, screen: u64) -> Wiring {
    assert!(n <= SCENE.len() && n <= compositor::MAX_WINDOWS && focusable <= n);
    let image = program("compositor").expect("no compositor program in the initrd archive");
    let client_image = program("window").expect("no window program in the initrd archive");
    let term_image =
        program("display_terminal").expect("no display_terminal program in the initrd archive");

    let wlist = zeroed_page_frame();
    let ring = zeroed_page_frame();
    let doorbell = crate::sched::create_rendezvous();
    let report = crate::sched::create_rendezvous();

    // **One contiguous run for every client's frames.** Not a convenience: it is what makes the
    // neighbour attack real, because it puts a client's neighbour's pixels in the frame physically
    // after its own grant. A test that attacked a scattered allocation would prove only that it
    // could not find the neighbour.
    let mut per_client = [0u64; compositor::MAX_WINDOWS];
    let mut total = 0u64;
    for (i, win) in SCENE.iter().take(n).enumerate() {
        per_client[i] = 1 + win.page_frames() as u64; // a control page, then the surface
        total += per_client[i];
    }
    // Zeroed so no client and no test ever reads a stale pixel.
    let run_base = crate::memory::alloc_contiguous_zeroed(total as usize)
        .expect("no contiguous run for the compositor's client surfaces")
        .addr();

    let mut client = [0u64; compositor::MAX_WINDOWS];
    let mut client_report = [0; compositor::MAX_WINDOWS];
    let mut input = [0; compositor::MAX_WINDOWS];
    let mut at = run_base;
    for i in 0..n {
        client[i] = at;
        at += per_client[i] * FRAME_SIZE;
        client_report[i] = crate::sched::create_rendezvous();
    }
    for ep in input.iter_mut().take(focusable) {
        *ep = crate::sched::create_rendezvous();
    }

    // The compositor's world: the list it publishes, the ring it reads, and every client's control
    // page and surface, all still `Spawn::maps` entries (small, fixed in number). **Not the screen**
    // (milestone 142, DECISIONS §102): at the grown scanout, `SCREEN_PAGE_FRAMES` (900) one-page
    // `Mapping` entries would no longer fit in the 1 KiB a spawn closure may capture
    // (`kernel/src/thread.rs`'s own limit), the same reason the display driver's DMA region moved
    // to a single run capability. The screen is granted below as one `PageFrame` instead, and the
    // compositor maps it itself. No device, no interrupt, no physical address.
    let mut maps = [Mapping {
        va: 0,
        phys: 0,
        flags: Flags::user_data(),
    }; MAX_COMP_MAPS];
    let mut m = 0;
    maps[m] = Mapping {
        va: WLIST_VA,
        phys: wlist,
        flags: Flags::user_data(),
    };
    m += 1;
    maps[m] = Mapping {
        va: RING_VA,
        phys: ring,
        flags: Flags::user_data(),
    };
    m += 1;
    for i in 0..n {
        for k in 0..per_client[i] {
            maps[m] = Mapping {
                va: CLIENT_BASE + i as u64 * CLIENT_STRIDE + k * FRAME_SIZE,
                phys: client[i] + k * FRAME_SIZE,
                flags: Flags::user_data(),
            };
            m += 1;
        }
    }

    let budget =
        crate::memory_region::create(MAP_BUDGET_PAGES).expect("no map budget for the compositor");

    let mut grants = [rendezvous_cap(report, Rights::WRITE); MAX_COMP_GRANTS];
    grants[1] = rendezvous_cap(display, Rights::WRITE);
    grants[2] = rendezvous_cap(doorbell, Rights::READ);
    // The whole screen, one capability (§102): `Object::PageFrame(screen, SCREEN_PAGE_FRAMES)`.
    grants[3] = page_frame_run_cap(
        screen,
        SCREEN_PAGE_FRAMES,
        Rights::READ.union(Rights::WRITE),
    );
    grants[4] = memory_region_cap(budget);
    for i in 0..focusable {
        grants[COMP_INPUT_BASE as usize + i] = rendezvous_cap(input[i], Rights::WRITE);
    }
    let ngrants = COMP_INPUT_BASE as usize + focusable;

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: n as u64,
                arg1: focusable as u64,
                arg2: 0,
                grants: &grants[..ngrants],
                maps: &maps[..m],
            },
        )
    })
    .expect("could not spawn the compositor");

    Wiring {
        doorbell,
        report,
        screen,
        wlist,
        ring,
        client,
        client_report,
        input,
        n,
        focusable,
        image: client_image,
        term_image,
        ring_tail: 0,
    }
}

impl Wiring {
    /// **Spawn window client `i` in `role`.** Its whole authority: a report endpoint, the doorbell,
    /// its own control page and surface, an input endpoint if it is focusable, and (only for
    /// [`ROLE_CAPTURE`]) a read-only mapping of the screen and the window list.
    pub fn spawn_client(&self, i: usize, role: u64) {
        let frames = SCENE[i].page_frames() as u64;
        let mut maps = [Mapping {
            va: 0,
            phys: 0,
            flags: Flags::user_data(),
        }; MAX_CLIENT_MAPS];
        let mut m = 0;
        maps[m] = Mapping {
            va: CTL_VA,
            phys: self.client[i],
            flags: Flags::user_data(),
        };
        m += 1;
        for k in 0..frames {
            maps[m] = Mapping {
                va: SURFACE_VA + k * FRAME_SIZE,
                phys: self.client[i] + (1 + k) * FRAME_SIZE,
                flags: Flags::user_data(),
            };
            m += 1;
        }
        // The window-list mapping stays a `Spawn::maps` entry (one page, unaffected by the
        // scanout's size). The screen itself does not (milestone 142, DECISIONS §102): see this
        // `impl`'s own note on `SCREEN_FRAME`/`BUDGET` below for why, and `fixtures/src/window.rs`'s
        // matching constants for the client side.
        if role & ROLE_CAPTURE != 0 {
            maps[m] = Mapping {
                va: C_WLIST_VA,
                phys: self.wlist,
                flags: Flags::user_rodata(),
            };
            m += 1;
        }

        let mut grants = [rendezvous_cap(self.client_report[i], Rights::WRITE); 3];
        grants[1] = rendezvous_cap(self.doorbell, Rights::WRITE);
        let ngrants = if i < self.focusable {
            grants[2] = rendezvous_cap(self.input[i], Rights::READ);
            3
        } else {
            // **Two grants, and the emptiness of slot 2 is load-bearing**: this client cannot
            // receive input, and its attempt to try is `NoSuchSlot` rather than a refusal from
            // anyone. See ROLE_PROBE_INPUT.
            2
        };

        // `SCREEN_FRAME`/`BUDGET` (slots 3/4, matching `fixtures/src/window.rs`): granted **only** for
        // `ROLE_CAPTURE`, at explicit slots via `grant_at` rather than through `Spawn.grants`'
        // sequential first-free fill, because slot 2 must stay genuinely empty for a non-focusable
        // client (the `ROLE_PROBE_INPUT` property above) and a sequential fill cannot skip it.
        // **This is still the kernel's decision, not the client's**: the capability is granted
        // here, by the spawner, based on `role`, exactly as the old `Spawn::maps` entry was. A
        // hostile `window` binary that ignored its own `role` argument still could not read the
        // screen unless *this* code decided to grant it, which is the same guarantee
        // `ROLE_PROBE_SCREEN`'s neighbouring test checks: nothing here lets a process's own code
        // grant itself authority the spawner withheld.
        //
        // The screenshot and enumeration grant is **read-only**: a thing that may look at the
        // screen may not draw on it. `Rights::READ` alone (no `WRITE`) is the difference between a
        // screenshot tool and a second compositor; `fixtures/src/window.rs`'s own `ROLE_CAPTURE` block
        // proves the write half faults.
        let capture_budget = if role & ROLE_CAPTURE != 0 {
            Some(
                crate::memory_region::create(MAP_BUDGET_PAGES)
                    .expect("no map budget for a capture client"),
            )
        } else {
            None
        };
        let screen = self.screen;

        let image = self.image;
        let probe = self.neighbour_probe_va(i);
        crate::sched::spawn(move || {
            if let Some(budget) = capture_budget {
                crate::sched::grant_at(
                    3,
                    page_frame_run_cap(screen, SCREEN_PAGE_FRAMES, Rights::READ),
                )
                .expect("client slot 3 was occupied");
                crate::sched::grant_at(4, memory_region_cap(budget))
                    .expect("client slot 4 was occupied");
            }
            run(
                image,
                Spawn {
                    arg0: role,
                    arg1: probe,
                    arg2: 0,
                    grants: &grants[..ngrants],
                    maps: &maps[..m],
                },
            )
        })
        .expect("could not spawn a window client");
    }

    /// **The address at which client `i`'s neighbour's pixels really are**, in `i`'s own virtual
    /// address space: one page past the last frame of its surface, which the contiguous allocation
    /// above makes the neighbour's control page, and one further is the neighbour's first pixel
    /// page. The attacker is handed this so the test can assert on the exact faulting address, the
    /// same way milestone 29's escape test is handed its victim frame.
    pub fn neighbour_probe_va(&self, i: usize) -> u64 {
        SURFACE_VA + (SCENE[i].page_frames() as u64 + 1) * FRAME_SIZE
    }

    /// The physical frame the probe address would reach if it were mapped. The test asserts this is
    /// the neighbour's first pixel frame, which is what makes the attack a real one.
    pub fn neighbour_probe_phys(&self, i: usize) -> u64 {
        self.client[i] + (SCENE[i].page_frames() as u64 + 2) * FRAME_SIZE
    }

    /// Client `i`'s surface, digested by the **kernel** through the direct map: a witness that
    /// belongs to nobody in userspace.
    pub fn client_surface_digest(&self, i: usize) -> u64 {
        let base = mmu::phys_to_virt(self.client[i] + FRAME_SIZE);
        compositor::surface_checksum(SCENE[i].w, SCENE[i].h, |k| {
            // SAFETY: inside the frames this kernel allocated for client `i`'s surface, reached
            // through the direct map.
            unsafe { core::ptr::read_volatile((base + (k * 4) as u64) as *const u32) }
        })
    }

    /// The composed screen, read by the kernel through the direct map.
    pub fn screen_pixel(&self, x: u32, y: u32) -> u32 {
        let at = mmu::phys_to_virt(self.screen) + (y * compositor::SCREEN_W + x) as u64 * 4;
        // SAFETY: inside the scanout frames, reached through the direct map.
        unsafe { core::ptr::read_volatile(at as *const u32) }
    }

    /// Write `v` into the screen at `(x, y)`: the poison a damage test needs, so that "the
    /// compositor did not touch this" is an observation rather than an inference.
    pub fn poison_screen_pixel(&self, x: u32, y: u32, v: u32) {
        let at = mmu::phys_to_virt(self.screen) + (y * compositor::SCREEN_W + x) as u64 * 4;
        // SAFETY: as above; the kernel owns these frames and no device is reading them in the
        // tests that poison (the display is a kernel stand-in there).
        unsafe { core::ptr::write_volatile(at as *mut u32, v) };
    }

    /// Which window the compositor says has focus, read out of the page it publishes. The
    /// compositor's decision, witnessed rather than asked for.
    pub fn focused(&self) -> u32 {
        let at = mmu::phys_to_virt(self.wlist) + compositor::proto::wlist::FOCUSED;
        // SAFETY: inside the window-list frame this kernel allocated.
        unsafe { core::ptr::read_volatile(at as *const u32) }
    }

    /// **Play the input driver**: put `bytes` in the ring and ring the doorbell.
    ///
    /// This is what a virtio-keyboard driver would do, and the authority it exercises is the ring
    /// mapping, not the doorbell: any client can ring, and none of them can write here. The CALL
    /// returns once the compositor has processed the frame, so a test needs no polling.
    pub fn type_bytes(&mut self, bytes: &[u8]) {
        let base = mmu::phys_to_virt(self.ring);
        for &b in bytes {
            let at = base
                + compositor::proto::ring::BYTES
                + (self.ring_tail % compositor::proto::ring::CAPACITY) as u64;
            // SAFETY: inside the ring frame this kernel allocated and shares with the compositor.
            unsafe { core::ptr::write_volatile(at as *mut u8, b) };
            self.ring_tail = self.ring_tail.wrapping_add(1);
        }
        // The bytes must be visible before the tail that advertises them.
        //
        // PAIR: `drain_input` in components/src/compositor.rs, which reads `TAIL` and then the bytes. This
        // is the **fourth** writer into pages that reader consumes, and milestone 43's audit counted
        // three: it named `window.rs`, `display_terminal.rs` and `keyboard_driver.rs` and missed the kernel
        // playing the same input-driver role here. Its fix covers all four, because `drain_input` is
        // the single reader. The `ipc_call` below also orders this one on its own (the compositor is
        // blocked in `recv_cap` on the doorbell), so the reader's fence is not what makes *this*
        // producer safe; see notes/memory-ordering.md for which producer it is.
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        // SAFETY: inside the ring frame.
        unsafe {
            core::ptr::write_volatile(
                (base + compositor::proto::ring::TAIL) as *mut u32,
                self.ring_tail,
            );
        };
        let w0 = compositor::proto::req(compositor::proto::COMMIT, 0);
        crate::sched::ipc_call(self.doorbell, [w0, 0]);
    }

    /// Ring the doorbell without typing anything: "look at the surfaces". Returns the reply's `r0`.
    ///
    /// The milestone tour uses it; the compositor tests all type something first.
    #[cfg_attr(test, allow(dead_code))]
    pub fn ring_doorbell(&self, op: u64) -> u64 {
        crate::sched::ipc_call(self.doorbell, [compositor::proto::req(op, 0), 0])[0]
    }

    /// **Spawn a display terminal as window `i`** (milestone 29's text increment).
    ///
    /// It takes a `window` client's place with **exactly a window client's authority**: a report
    /// endpoint, the doorbell, an input endpoint, and its own control page and surface. The
    /// compositor cannot tell it from the client that painted a coordinate pattern, which is the
    /// same claim rung two made about `gpu_driver` one seam down, now made about a client.
    ///
    /// The one addition is an **output page**, and it belongs to the terminal contract rather
    /// than to the compositor's: it is where an application puts the bytes of an `OP_WRITE`
    /// (DECISIONS §10). The kernel holds the other end of that, playing the application.
    ///
    /// **Its input endpoint and its terminal endpoint are the same endpoint**, deliberately.
    /// This process has one wait point (DECISIONS §33), so an application printing and the
    /// compositor typing must arrive on one endpoint and be told apart by opcode, exactly as
    /// `line_editor` does for the serial terminal. The compositor holds WRITE on it because window `i`
    /// is focusable; the kernel holds it too, as the spawner.
    ///
    /// Returns the output page's physical address. `i` must be below `focusable`, or the
    /// terminal would hold no endpoint to serve and would park forever on its first receive.
    pub fn spawn_terminal(&self, i: usize) -> TermClient {
        assert!(
            i < self.focusable,
            "a display terminal must be focusable: its input endpoint is the endpoint it serves",
        );
        // The window must hold at least one character cell. It need not be a whole number of
        // them, for the reason the scanout is not: the font is 7 wide and no window here is a
        // multiple of 7, so each leaves a strip on the right that its terminal paints as
        // background on the first frame rather than leaving as whatever the frame held.
        assert!(
            SCENE[i].w >= bitmap_font::GLYPH_W && SCENE[i].h >= bitmap_font::GLYPH_H,
            "window {i} is too small for one character cell",
        );

        let out = crate::memory::alloc_zeroed()
            .expect("no output-page frame for a display terminal")
            .addr();

        let frames = SCENE[i].page_frames() as u64;
        let mut maps = [Mapping {
            va: 0,
            phys: 0,
            flags: Flags::user_data(),
        }; MAX_CLIENT_MAPS];
        let mut m = 0;
        maps[m] = Mapping {
            va: T_CTL_VA,
            phys: self.client[i],
            flags: Flags::user_data(),
        };
        m += 1;
        maps[m] = Mapping {
            va: T_OUT_VA,
            phys: out,
            flags: Flags::user_data(),
        };
        m += 1;
        for k in 0..frames {
            maps[m] = Mapping {
                va: T_SURFACE_VA + k * FRAME_SIZE,
                phys: self.client[i] + (1 + k) * FRAME_SIZE,
                flags: Flags::user_data(),
            };
            m += 1;
        }

        let grants = [
            rendezvous_cap(self.client_report[i], Rights::WRITE),
            rendezvous_cap(self.doorbell, Rights::WRITE),
            rendezvous_cap(self.input[i], Rights::READ),
        ];
        let image = self.term_image;
        crate::sched::spawn(move || {
            run(
                image,
                Spawn {
                    arg0: video_terminal::status::MODE_WINDOW,
                    arg1: 0,
                    arg2: 0,
                    grants: &grants,
                    maps: &maps[..m],
                },
            )
        })
        .expect("could not spawn a display terminal");

        TermClient {
            out,
            ep: self.input[i],
        }
    }
}

// A display terminal's address space. Must match components/src/display_terminal.rs. Different numbers from a
// `window` client's, because they are different programs; the kernel picks each binary's.
// `T_OUT_VA`/`T_CTL_VA` match `display_terminal.rs`'s own moved constants (milestone 142): that
// binary uses the same three addresses in both `MODE_DISPLAY` and `MODE_WINDOW`, so moving them
// for `MODE_DISPLAY`'s grown surface moves them here too, even though `MODE_WINDOW`'s own surface
// (this function's `frames`, window-sized) never grew and never collided on its own.
const T_SURFACE_VA: u64 = address_space_map::pair_page(0x0000_0000_0060_0000);
const T_OUT_VA: u64 = address_space_map::pair_page(0x0000_0000_0a00_0000);
const T_CTL_VA: u64 = address_space_map::pair_page(0x0000_0000_0a01_0000);

/// A display terminal running as a compositor client, from the spawner's side: the page it reads
/// an application's bytes out of, and the endpoint it serves.
pub struct TermClient {
    pub out: u64,
    pub ep: crate::sched::RendezvousId,
}

impl TermClient {
    /// Play the application: `OP_WRITE` this text and return when it is on the screen.
    pub fn print(&self, text: &[u8]) {
        super::term_print(self.out, self.ep, text);
    }
}

/// The most mappings a compositor can need: the list, the ring, and every client's control page
/// and surface. The screen is **not** here since milestone 142 (§102): it is a `PageFrame` grant,
/// not a `Spawn::maps` entry (see `start`'s own comment).
const MAX_COMP_MAPS: usize = 2 + compositor::MAX_WINDOWS * 4;
/// The most a client can need: its control page, its surface, and (capture only) the window list.
/// The screen is **not** here for the same reason as `MAX_COMP_MAPS`: a capture client maps it
/// itself, out of the `SCREEN_FRAME`/`BUDGET` grants `fixtures/src/window.rs` holds.
const MAX_CLIENT_MAPS: usize = 4 + 1;
/// Report, display, doorbell, the screen `PageFrame`, its map budget, then one input endpoint per
/// focusable client starting at [`COMP_INPUT_BASE`].
const MAX_COMP_GRANTS: usize = COMP_INPUT_BASE as usize + compositor::MAX_WINDOWS;
/// The first of the compositor's per-client input-endpoint grant slots. Must match
/// `components/src/compositor.rs`'s own `INPUT` constant.
const COMP_INPUT_BASE: u64 = 5;

/// A fresh zeroed frame, for a page the kernel hands two processes to share.
fn zeroed_page_frame() -> u64 {
    crate::memory::alloc_zeroed()
        .expect("no frame for the compositor's shared pages")
        .addr()
}
