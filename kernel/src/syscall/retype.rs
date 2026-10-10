//! **`MemoryRegion::RETYPE_OBJ`**: a kernel object made in a region's memory, and the capability to
//! it filed in the caller's table. Moved out of `syscall.rs` unchanged but for the floor, by
//! milestone 812 (`std::thread::spawn` runs real threads in one address space), for §266 (a Rust
//! source file stays under 2,000 lines).
//!
//! **The third word is a floor** (milestone 812, provisional): the capability lands in the first
//! free slot at or above it, and `0` is the first free slot as before. A `std` program's low slots
//! are fixed by number and an empty one means "not granted" (`std_runtime_protocol`), so what it
//! makes for itself, a thread or a timer, must land above them.
//!
//! Name: provisional (milestone 812's lane, 2026-10-10 UTC).

use abi::Error;

use crate::cap::Rights;
use crate::sched;

/// `MemoryRegion::RETYPE_OBJ`: retype a page into a page-resident KERNEL OBJECT the caller now owns
/// (19a). The object lives in the carved page, the region is pinned (a live endpoint's page must
/// never be freed under a blocked thread), and the caller gets full rights on its own object,
/// delegation narrowing them as ever. `#[inline(never)]` for the reason `memory_region_map` gives.
#[inline(never)]
pub(super) fn memory_region_retype_obj(
    region: u64,
    kind: u64,
    space_slot: u64,
    floor: u64,
) -> Result<i64, Error> {
    match kind {
        abi::objtype::RENDEZVOUS => {
            let ep = sched::create_rendezvous_from(region).ok_or(Error::OutOfMemory)?;
            // `Rights::ALL`, not a list. The comment above has always said the creator gets full
            // rights on its own object; spelling the set out meant "full" silently stopped being
            // full the day `ENUMERATE` was added, and the symptom was three steps away: the progenitor
            // could not narrow `deaths` to a right it did not itself hold, `CAP_INSERT` refused
            // the widen, and the spawn surfaced as `OutOfMemory` at a prompt. A rights set that
            // must be updated by hand whenever a right is added is rung four; `ALL` is the
            // invariant.
            let slot = sched::grant_from(floor, crate::cap::rendezvous_cap(ep, Rights::ALL))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // An address space (19b): the page becomes the L0 root, the untyped becomes the space's
        // backing region for tables and records (one budget model; see the abi doc and
        // design/init-and-granular-spawn.md).
        abi::objtype::ADDRESS_SPACE => {
            let name = crate::user::user_address_space_create(region).ok_or(Error::OutOfMemory)?;
            // `Rights::ALL` for the RENDEZVOUS arm's reason: "full rights on its own object" is the
            // invariant, and a hand-listed set stops being full the next time a right is added.
            // `AddressSpace` does not consult `ENUMERATE` today and is expected to when `pmap` is
            // built; holding a right nothing checks confers nothing, and not holding it is what
            // blocks a future grant.
            let slot = sched::grant_from(floor, crate::cap::address_space_cap(name, Rights::ALL))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // A thread (19c.3): the page holds an embryo TCB, born in no queue and not runnable
        // until CONFIGURE + START. The page is the creator's region's.
        abi::objtype::THREAD_CONTROL_BLOCK => {
            let tid = sched::create_thread_control_block(region).ok_or(Error::OutOfMemory)?;
            let slot = sched::grant_from(
                floor,
                crate::cap::thread_control_block_cap(tid, Rights::ALL),
            )
            .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // A notification (milestone 151, DECISIONS §101): a word and a wait queue in the page.
        // `Rights::ALL` for the RENDEZVOUS arm's reason.
        abi::objtype::NOTIFICATION => {
            let id = sched::create_notification_from(region).ok_or(Error::OutOfMemory)?;
            let slot = sched::grant_from(floor, crate::cap::notification_cap(id, Rights::ALL))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // A timer (milestone 106, DECISIONS §147): one deadline and its target in the page.
        // `Rights::ALL` for the RENDEZVOUS arm's reason.
        abi::objtype::TIMER => {
            let id = sched::create_timer_from(region).ok_or(Error::OutOfMemory)?;
            let slot = sched::grant_from(floor, crate::cap::timer_cap(id, Rights::ALL))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // A process (milestone 812 (`std::thread::spawn` runs real threads in one address space)),
        // holding the space whose `WRITE` capability is in `space_slot`, which is consumed as
        // `CONFIGURE` consumes one. `Rights::ALL`, `BIND` among them, for the RENDEZVOUS arm's
        // reason.
        abi::objtype::PROCESS => {
            let space = sched::current_cap(space_slot).map_err(|_| Error::NoSuchSlot)?;
            let crate::cap::Object::AddressSpace(name) = space.object else {
                return Err(Error::WrongObject);
            };
            if !space.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            let pid = sched::create_process_from(region, name)?;
            let _ = sched::delete_current_cap(space_slot);
            let slot = sched::grant_from(floor, crate::cap::process_capability(pid, Rights::ALL))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        _ => Err(Error::BadMethod), // no such object type
    }
}
