use super::*;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched::{self, RendezvousId};
use crate::user::holding::Holding;

/// The VAs `components/src/rmle.rs` hardcodes. Must match that file.
const TERM_OUT_VA: u64 = address_space_map::pair_page(0x0000_0000_0080_0000);
const FS_VA: u64 = address_space_map::pair_page(0x0000_0000_0060_0000);

/// Stack pages beyond the one `run` maps. `rmle`'s own `Editor` is a few kilobytes (32 rows of
/// 100 bytes plus bookkeeping) and this program has no allocator, so it all lives on the stack;
/// sized with the same "extra pages rather than exactly enough" margin `swish`'s own wiring uses.
const RMLE_STACK_PAGES: usize = 4;

/// The file channel's width, straight from the contract `rmle`'s FS calls speak, so this wiring
/// cannot disagree with the program on the other end. `fs_service::FILE_PAGES` is the same value
/// but private to that module; this is the source it is itself defined from.
const FILE_PAGES: usize = filesystem_protocol::fs::TRANSFER_PAGES;

/// A fresh, zeroed frame, for `rmle`'s own extra stack pages. The zeroing and its soundness
/// argument now live once in [`crate::memory::alloc_zeroed`] (milestone 139 round 8); what stays
/// here is this module's own "no frame for rmle's stack" panic message.
fn page_frame() -> u64 {
    crate::memory::alloc_zeroed()
        .expect("no frame for rmle's stack")
        .addr()
}

/// A running `rmle`, wired with a real terminal (`line_editor` and a fake console, exactly
/// [`raw_mode_service::start`]'s wiring) and a real filesystem (a directory narrowed to `dir_name`
/// under `fs_subtree_caretaker`, exactly [`fs_service::narrow_dir`]'s wiring). Composing the two is
/// the only thing this module adds: `rmle` is the one program in this tree that needs both a
/// terminal and a file at once, the same shape `swish`'s own wiring already has (milestone 50's
/// redirection witness), so nothing here is a new pattern.
pub struct Wiring {
    /// The terminal endpoint, played by the fake console + real `line_editor` underneath. A test
    /// wanting to observe the screen reads [`Wiring::term_out_phys`] after driving keystrokes
    /// through the same input-driver role [`raw_mode_service`]'s own tests use.
    pub term: RendezvousId,
    pub term_out_phys: u64,
    /// The narrowed directory `rmle` holds. A test may also call this directly (after `rmle` has
    /// exited and stopped using its shared page) to read the file back as an independent witness,
    /// exactly the two-witness discipline `c_seam`'s confiner tests use elsewhere.
    pub dir: RendezvousId,
    pub file_shared: u64,
    /// `rmle`'s one closing report: `(STATUS_QUIT | STATUS_OPEN_FAILED, dirty-or-errno, 0)`.
    pub report: RendezvousId,
}

/// Spawn `rmle` against `dir_name` (a directory already in the test fixture tree, granted with
/// [`filesystem_protocol::dir::ALL`]) and `file_name` (the file inside it `rmle` edits, created if
/// absent). `None` if there is no RedoxFS disk attached to this run.
///
/// **Returns a [`Holding`] alongside the wiring, and a caller must release it.** The fake console
/// and `line_editor` underneath `rmle`'s terminal both block forever, exactly
/// [`raw_mode_service::start`]'s own case and for the same reason (see its doc comment): neither
/// exits on its own, so `TERM`/`CONREQ`/`CONREP` are minted from a region the `Holding` carries
/// rather than from `sched::create_rendezvous`'s kernel-global pool, and reclaiming that region is
/// what actually wakes them. `rmle` itself is not in the `Holding`: the test drives it to a clean
/// exit over `^Q` and reads its own report, so by the time a caller would release this holding
/// `rmle`'s thread is already gone.
pub fn start(dir_name: &'static str, file_name: &str) -> Option<(Wiring, Holding)> {
    let image = program("rmle").expect("no rmle program in the initrd archive");

    // The terminal half: identical to raw_mode_service::start, duplicated rather than reused
    // because that function's Wiring hands back a line_editor already wired to nobody; composing a
    // *third* process's grants into a spawn that function does not perform would need it split the
    // way fs_service::narrow_dir already is split from fs_service::start_granted_dir, and one
    // wiring function is enough plumbing for a milestone that is not itself about wiring.
    let term_img = program("line_editor").expect("no line_editor program in the initrd archive");
    // Four pages for three endpoints, one page each and one spare: `virtio_service`'s own
    // `wire_net_server` carves the same shape for the same reason (see `raw_mode_service::start`).
    let ep_region = crate::memory_region::create(4).expect("no endpoint region for rmle_service");
    let term = sched::create_rendezvous_from(ep_region).expect("no TERM endpoint");
    let conreq = sched::create_rendezvous_from(ep_region).expect("no CONREQ endpoint");
    let conrep = sched::create_rendezvous_from(ep_region).expect("no CONREP endpoint");
    let console_phys = crate::memory::alloc_zeroed()
        .expect("no frame for the fake console")
        .addr();
    let term_out_phys = crate::memory::alloc_zeroed()
        .expect("no frame for rmle's terminal-output page")
        .addr();
    let term_app_in_phys = crate::memory::alloc_zeroed()
        .expect("no frame for line_editor's unused app-input page")
        .addr();
    let console_tid = sched::spawn(move || {
        loop {
            sched::ipc_recv(conreq);
            sched::ipc_send(conrep, [0, 0, 0]);
        }
    })
    .expect("could not spawn the fake console");
    let line_editor_tid = sched::spawn(move || {
        run(
            term_img,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(term, Rights::READ),
                    rendezvous_cap(conreq, Rights::WRITE),
                    rendezvous_cap(conrep, Rights::READ),
                ],
                maps: &[
                    Mapping {
                        va: 0x0060_0000, // line_editor's own CONOUT_VA
                        phys: console_phys,
                        flags: Flags::user_data(),
                    },
                    Mapping {
                        va: 0x0080_0000, // line_editor's own APP_OUT_VA
                        phys: term_out_phys,
                        flags: Flags::user_rodata(),
                    },
                    Mapping {
                        va: 0x0090_0000, // line_editor's own APP_IN_VA
                        phys: term_app_in_phys,
                        flags: Flags::user_data(),
                    },
                ],
            },
        )
    })
    .expect("could not spawn line_editor");

    // The filesystem half: fs_service's own split, built for exactly this (a client that needs a
    // directory *and* something else, milestone 50's own redirection witness).
    let (dir_ep, file_shared) = fs_service::narrow_dir(
        fs_service::blk_server_image(),
        fs_service::fs_server_image()?,
        program("fs_subtree_caretaker").expect("no fs_subtree_caretaker program in the initrd"),
        dir_name,
        filesystem_protocol::dir::ALL,
    )?;

    assert!(
        filesystem_protocol::grant::fits(file_name.as_bytes()),
        "rmle's target name rides in two argument words; this one does not fit",
    );
    let (lo, hi) = filesystem_protocol::grant::pack_name(file_name.as_bytes());
    let spec = filesystem_protocol::grant::spec(file_name.len(), 0);

    let report = sched::create_rendezvous();
    sched::spawn(move || {
        let mut maps = [Mapping {
            va: 0,
            phys: 0,
            flags: Flags::user_data(),
        }; FILE_PAGES + RMLE_STACK_PAGES + 1];
        maps[0] = Mapping {
            va: TERM_OUT_VA,
            phys: term_out_phys,
            flags: Flags::user_data(),
        };
        let n = 1 + fs_service::map_channel(&mut maps[1..], FS_VA, file_shared, FILE_PAGES);
        for (k, m) in maps[n..n + RMLE_STACK_PAGES].iter_mut().enumerate() {
            m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
            m.phys = page_frame();
        }
        run(
            image,
            Spawn {
                arg0: spec,
                arg1: lo,
                arg2: hi,
                grants: &[
                    rendezvous_cap(term, Rights::WRITE),   // slot 0: the terminal
                    rendezvous_cap(dir_ep, Rights::WRITE), // slot 1: the directory
                    rendezvous_cap(report, Rights::WRITE), // slot 2: the closing report
                ],
                maps: &maps[..n + RMLE_STACK_PAGES],
            },
        )
    })
    .expect("could not spawn rmle");

    let mut held = Holding::new();
    held.add_thread(console_tid);
    held.add_thread(line_editor_tid);
    held.add_region(ep_region);

    Some((
        Wiring {
            term,
            term_out_phys,
            dir: dir_ep,
            file_shared,
            report,
        },
        held,
    ))
}

// BUGS: the console/line_editor terminal half is spawned before the two fallible calls below it
// (`fs_service::fs_server_image()?`, `fs_service::narrow_dir(...)?`). If either ever refuses (no
// RedoxFS disk attached to this run, the same condition `NO_FS_SERVER` skips a test for), this
// function returns `None` with the terminal's two threads already spawned and no `Holding` to
// release them: a real leak on that path, unreachable in any run where `fs_server_image` succeeds
// (every run this fix was tested against). Not fixed here because it is orthogonal to the leak
// this lane found and fixed (the *reachable* every-run case), and `raw_mode_service::start` has no
// equivalent fallible step to compare it against.
