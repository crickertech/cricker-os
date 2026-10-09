# Boot wiring and measured boot: the history the login module doc carried

<!-- writing-standards: exception. Marked by milestone 860 (comments state the constraint as it is
now) on 2026-10-09 (UTC). Reason: this file is a verbatim historical record moved out of a Rust
module doc, whose sentences were written to that medium's rules and none of these; rewording them
to meet the median would edit the record rather than move it. Splitting these sentences is fair
game for any later lane that touches them for a reason of its own. -->

Moved 2026-10-09 (UTC) from `components/src/login.rs`'s module doc by milestone 860 (comments
state the constraint as it is now), so the doc could state only the constraints as they are. The
entries below are the resolved records, kept with their dates.

## Resolved, milestone 233 (`login` dies on every boot) (2026-09-02): this program used to die at `_start` on every real
interactive boot, on both architectures

`_start` read the boot archive to find `fs_subtree_caretaker`, from `initrd_len` in `a1` and the
kernel's mapping at `user_mode_runtime::initrd::INITRD_VA`. That is what
`kernel::user::login_service::start` handed it, and it is what `crates/system_initializer` could
never hand it: `supervision_protocol::build_child` maps only pages the spawner holds a
`PageFrame` capability for, and the archive is reserved RAM the frame allocator does not own and
no capability names. So the progenitor started this process with `start_child(login_child, 0, 0,
0)` and no archive, `initrd_bytes` yielded a zero-length slice, `nifefs::Fs::parse` refused it,
and this process took `fail(1)` before serving anything, while the boot went on printing `init:
login ready` with a generated password.

The real defect was that the two spawners disagreed, and only one of them was tested. A harness
that starts a program differently from the way the system starts it is not testing that program,
and this one had been passing for an unknown length of time. Both now lay down the same two blobs
(the caretaker ELF and the measurement table), and `login_protocol::CARETAKER_ELF_VA` carries the
account of why the fix went this way rather than by finding some way to give this process the
archive: it needs one program's bytes and a table to check them against, and a service that can
read every file in the boot image to answer a password holds authority it never exercises.

Two things are gone with it. This process no longer refuses to start over the caretaker at all:
absent, unparseable and unvouched-for all become `care_elf = None` and a `DENIED` per login, which
is the posture `crates/system_initializer` already had toward this exact component. And `init:
login ready` is now `progenitor: login credentials provisioned`, which is what the first process
actually measured; the survival claim moved to `script/swish-check`, which fails if the kernel
reported killing any user thread during the run.

What this cost in the currency that was scarce: nothing. Milestone 231 (nothing counts how many capability slots a boot actually uses)'s gauge says the boot's
capability-slot high-water mark is 21 of 24 before this change and 21 of 24 after it, on both
architectures, because `supervision_protocol`'s `fill_and_map` holds one frame capability at a
time and deletes it.

The measurement check's trust root moved and is weaker. When this process read the initrd it read
the same physical archive the kernel maps for the progenitor, so the check was independent of
whoever spawned it. Under `crates/system_initializer` both the bytes and the table now arrive from
the progenitor, which has already run the identical `measured_boot::verify_in_manifest` over them,
so what remains is a consistency check on the hand-over rather than an independent verification.

How long the original defect had been live is unknown and nobody bisected it. The audit trail did
carry which step refused (`fail`'s `0xDEAD_0000_0000_0000 | step`) and `login_audit_receiver`
discards it, which is that program's own recorded limitation and is part of why nobody saw this.

## Resolved, 2026-08-24: loading the caretaker used to be unchecked

This process used to load `fs_subtree_caretaker` by name with no check at all, inconsistent with
`crates/system_initializer`'s own discipline (milestone 104 (the measurement continues past init): refuse a program whose bytes do not
match the archive's measurement table). Investigating "how a non-progenitor loader joins that
chain" (the open question this BUGS entry used to leave unanswered) found the premise did not
hold: this process maps the *same physical archive* the kernel already maps for
`system_initializer`, the same read-only way, at the same address (`kernel::user::spawn_init` for
aarch64's progenitor, `kernel::user::login_service`'s own `start` for this process, both taking
the physical range from `memory::initrd_region()`), so the kernel's boot already vouches for this
process's copy exactly as much as it vouches for the progenitor's. There is no new trust boundary
to cross; milestone 233 then moved the bytes to a hand-over, which is the weaker trust root named
above.

The fold into `DENIED` is not the anti-oracle reasoning the other two folded cases get, and should
not be read as one. A wrong password and a missing subtree both vary with what a caller presents,
so folding them prevents a caller from learning something about a specific identity by comparing
outcomes across attempts. A failed caretaker measurement varies with *nothing* a caller controls:
the archive is immutable RAM fixed for the whole boot, so every identity, on every attempt, for
the rest of this process's life, gets the identical answer. There is nothing to probe. The honest
reason for the fold is narrower and more mundane: `login_protocol` has no separate wire code for
"this service's core dependency failed to verify," and `DENIED`'s own doc already covers "the
service could not mint a capability set for an otherwise-authenticated principal," which this is
one more instance of. The cost this leaves unaddressed is operational rather than a security gap:
an operator whose build produced a tampered or unmeasured `fs_subtree_caretaker` sees every real
login denied with no signal that the *cause* is the caretaker rather than, say, a misconfigured
credential store, and the audit endpoint does not help (it only records a *successful* login). A
deployment that wants that distinguished needs an operator-facing log distinct from the login
result, which this slice does not build.

Proven by
`kernel::user::login_tests::logins_caretaker_measurement_matches_the_real_table_and_a_tampered_one_would_be_refused`:
the real archive's `fs_subtree_caretaker` bytes verify against the real measurement table (so
`wired()`'s own instance, and every other test in that file, depends on this check passing), and
the identical `measured_boot::verify_in_manifest` call `_start` now makes refuses a tampered copy
and a name the table does not mention. That test cannot spawn a second login instance against a
deliberately corrupted archive to prove the wire-level `DENIED` end to end: the initrd is one
physical region the whole kernel test binary shares, set up once at boot, and `cargo xtask` always
packs a table that agrees with the bytes it just packed, so no test in this suite can make the
real archive disagree with itself. Proving the exact check `_start` performs, against the exact
name and table it uses, is the strongest proof available without a second, deliberately-tampered
kernel image, which was out of scope for that fix.

## Resolved, 2026-08-27: the terminal, executing the roadmap's recorded recommendation

`design/roadmap/0049-users-and-attribution.md`'s own BUGS already recorded the recommendation;
this update executed it rather than deciding it fresh. Milestone 49 (users and attribution)'s own text names three things
a login hands back (a directory, a budget, a terminal); this program used to hand back two. The
reason "hand one back" was ever a real design question rather than an unbuilt feature: a terminal
in this system is a singleton hardware-backed resource, wired once at interactive boot
(`crates/system_initializer::boot`), so handing it to a login-authenticated principal has to say
what a *second* concurrent login gets told, which naming a fourth capability slot does not answer
by itself. The shape built is the narrow one, matching what this boot actually is; what that
recommendation explicitly declined to build (a real multiplexing primitive, more than one live
session with its own view) remains undecided and unbuilt, on purpose.

## The device-grant half of the boot-prompt blocker, built 2026-08-26

Investigated 2026-08-23 (milestone/49-login-boot-prompt), built 2026-08-26 on §120 (a QEMU-only virtio-rng stopgap for the interactive boot)'s amendment.
§120 reversed its own 2026-08-23 decline ("calef is that customer, for a reason specific to this
project's own method": a QEMU boot is reachable unattended, a real board is not).
`kernel::user::spawn_init` (aarch64) and `kernel::user::riscv_shell_boot` (riscv64) now discover a
real virtio-rng device (MMIO only; the PCIe transport `kernel::user::entropy_service::start` also
offers is real follow-on, not built there) and grant it to the progenitor as three capabilities
(`BootEndowment::virtio_rng`/`virtio_rng_irq`/`virtio_rng_dma`), and the interactive boot's own
QEMU invocation now attaches one (`xtask`'s `swish_check_leg` and `shell` command both set
`NIFE_RNG`, where before it was a test-leg-only flag). `crates/system_initializer::boot` builds a
real entropy service from that grant, at the very top of the function, and proves it drew real
device bytes before building anything else (`script/swish-check` now reads `progenitor: entropy
service up; drew real bytes from a virtio-rng device` on both ISAs; it said `init:` until
milestone 266 (one progenitor on all three architectures) renamed the program). This is the harder half of the original blocker, and it
required no help from this program: `credentialer.rs` and `entropy.rs` are unmodified, because the
entropy service they both already assumed now genuinely exists under a real boot.

Why entropy had to be built before the console, and that ordering is load-bearing rather than
tidy: the virtio-rng trio is granted by the kernel, at spawn, so it inflates the progenitor's
resting capability-table baseline for the *whole* function, and the earliest peak `boot` ever
reaches (retyping the terminal's six capabilities, before the console is even built) was already
close to the wall on its own account. Building entropy anywhere after that peak, including in the
reasonable-looking gap the console's own three capabilities free, pushes it over and boots in
total silence; building it first, and releasing its three slots before the terminal plumbing ever
runs, restores every peak downstream to exactly what it was before this landed. Found by
bisection: an isolated test granted the progenitor one single harmless extra capability, unused by
any code, and the identical silent fault reproduced. See `crates/system_initializer::boot`'s own
comment on this block for the full account.

## The three pieces still missing, and the open password fork

Reaching this program, and a real credential, from the prompt needs three more pieces, none of
them plumbing gaps in the sense the device grant was:

1. `credentialer` and this program, wired into `boot` the same way entropy now is (built via
   `build_child`, holding narrowed views of capabilities `boot` already retypes or is granted: the
   file service pair, a construction budget, and the entropy service's own request endpoint, which
   `boot` would keep a client view of alongside the one it hands `credentialer`).
2. A real subtree and a real credential for whoever is meant to log in, which
   `identity_provisioner` (milestone 155 (a provisioning tool)) already builds the tool for, but that tool has the
   identical "spawned only by the kernel's guest test harness" bound this program's own BUGS used
   to name (`design/roadmap/0155-*`'s own BUGS, unchanged). Wiring it in is the same shape of
   `build_child` call as (1); it is listed separately because it raises the next point.
3. Where the demo credential's password comes from, which is a real, undecided fork and not a
   wiring detail: nothing today provisions a subtree or a credential for a real boot, and a lane
   should not silently choose one. Two shapes were considered, not built: a password baked into
   the image at build time (rejected there as a recommendation, not decided against absolutely: a
   fixed secret shipped in a public repository is exactly the "a fact that leaves the machine"
   category AGENTS.md's own tenet reserves for an architect, and it is also the harder one to
   undo); and a password the boot itself generates, from the entropy service already built there,
   provisioned once per boot and printed to the console before the prompt (in the shape cloud
   images already use for a generated first-boot password). The second is the recommendation: it
   needs no permanent secret, no decision about *whose* password to bake in, and it is reversible
   in exactly the sense that makes it a lane's call rather than calef's under the *move fast on
   what can be undone* tenet. Not built there because it is new work on top of (1) and (2), not
   because it is undecided in the sense that would block a lane from attempting it.

These live one level up, in `design/roadmap/0049-users-and-attribution.md`'s own BUGS, because
they are facts about the boot's wiring and milestone 155's own tool, not about what this program
does or does not do; this program's own contract is unchanged by any of them.
