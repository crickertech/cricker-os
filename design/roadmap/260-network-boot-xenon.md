---
status: PARTIAL
raised: 2026-09-05
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: xenon (its firmware settings and the house network it boots from are the work)
needs_person: yes
---
# 260. Boot xenon over the network, because the stick is the tax and the router is ours

Minted 2026-09-05 by calef, in one sentence: *"xenon already hurts. We can
control our router since it runs OpenWRT."* *(Number provisional until the merge queue lands it.)*
Built 2026-09-05: everything that does not need xenon or the house router, which is more of it than
this block predicted. What remains is one bench session and one router edit, both calef's, and
both now written down as procedures rather than as intentions.

In the second sense `notes/roadmap.md` names: xenon is here, the
router is here, and what remains is a person standing at both. Two firmware settings (tick `Enable
UEFI Network Stack`, move Integrated NIC to `Enabled w/PXE`), one router edit, one power cycle and
one photograph. They are calef's because they are his machine and his house network, not because
anything is unchosen. Everything else is a lane's, and none of it needed the machine.

The token was `DECISION` from 2026-09-05 to 2026-09-19, and it is corrected here rather than
quietly because it named the wrong kind of wait for fifteen days. Nothing in this block asks calef
to choose between options: the settings are named, the photographs that show them are cited, the
`dnsmasq` lines are written and a gate (`script/netboot-rehearsal --check`) already parses them. A
reader asking *what would change this* got "a decision" and the true answer is "an evening at the
bench", which is exactly the conflation calef corrected on 2026-08-18 when milestones 16 and 53 sat
on the ready list after the VisionFive 2 arrived. `DECISION` and `HARDWARE` both keep a `PARTIAL`
block off `script/roadmap --ready`, so nothing was mis-offered; what was wrong is what the gate
told a reader.

## Why this is not the same problem radon had, and why the answer just changed

Milestone 257 needed no cooperation from the network at all. U-Boot takes the server address
from a script we write onto the card, so `192.168.8.216` is baked in and the house router never has
to know anything.

PXE is the opposite. The DHCP server has to advertise the next server and the boot filename, and
that is the router. When 257's lane refused `dnsmasq`, it did so for exactly this reason, and its
own words are worth keeping because they are what made this look blocked: *"a second DHCP server on
it is an outage for everyone in the building."*

That objection does not apply here and the difference is the whole reason this is startable.
This is not a second DHCP server; it is one option on the existing one. OpenWRT's DHCP *is*
`dnsmasq`, and its boot options can be scoped to a single MAC address, so xenon gets an offer
and nothing else on the house network sees any change.

## What makes it easier than radon, which is the opposite of what anyone would guess

xenon's payload is one file. `cargo xtask uefi-image` stages the loader, the kernel and the
archive into a single `BOOTX64.EFI`, 9,210,880 bytes as measured on 2026-09-05 (this block said
9,180,160 when it was minted; the number moves with every kernel change and is not a constant). So a
netboot is one TFTP transfer, where radon needs two and a boot script to sequence them. There is no
fallback branch to write, because PXE either produces the file or the firmware moves to the next
boot entry on its own.

And `script/board-netboot` is already board-agnostic. It serves a directory over TFTP and
knows nothing about RISC-V; milestone 257 built it, and the ratified name says netboot rather than a
board or a protocol precisely so it could serve this too. **It served this without a single change
to how it serves radon**, at `--root target/esp`, and the one change it did need is in BUGS below.

## What was built

### The router half, written down and then made to run

`bench/xenon-netboot/dnsmasq.conf`. Four directives, each with its reason beside it: a `dhcp-host`
tagging xenon by MAC, two `dhcp-match` lines tagging an EFI x86-64 client by option 93, and one
`dhcp-boot` requiring both tags.

It is not only a record, and that is the point. `script/netboot-rehearsal` parses this file and
answers DHCP out of it, so a typo in the configuration fails a gate on patagonia rather than at a
bench with a camera in hand. That is the same move the multicast DNS responder's configuration document made (retired
2026-09-15; notes/mdns.md): one document, read by the thing and by the thing's test. A fact that lived only in the router would be the
rung-four failure AGENTS.md names, and a reflashed router would take it with it.

No address is assigned to xenon, deliberately: `set:` with no IP is valid dnsmasq, tags the host
without removing it from the dynamic range, and so cannot collide with a lease the router has
already handed out. Nothing needs to reach xenon at a known address.

### The architecture question, answered rather than sidestepped

DHCP option 93 says what the client can execute. Scoping by MAC alone would sidestep the problem,
and the hole that leaves is worth naming: **xenon itself is a BIOS client whenever `Boot List
Option` is Legacy**, which is one setup visit away. So the offer requires the architecture tag as
well, and a client in the wrong mode gets *no* boot filename and falls through to the next boot
entry, rather than being handed 9 MB of the wrong instruction set.

Measured, `script/netboot-rehearsal --check`, in under a second and with no emulator:

| Client | option 93 | Offered |
|---|---|---|
| xenon in UEFI mode | 7 | `EFI/BOOT/BOOTX64.EFI` from 192.168.8.138 (was .216; corrected 2026-10-04) |
| xenon, firmware sending the other EFI x86-64 value | 9 | the same |
| xenon after somebody sets Boot List Option to Legacy | 0 | nothing |
| xenon asking for an HTTP boot | 16 | nothing |
| xenon on firmware too old to send option 93 | absent | nothing |
| any other machine in the house | 7 | nothing |

Only the first row can be exercised under OVMF, because OVMF is a UEFI machine and cannot be asked
to be a BIOS one. That is why the table exists as its own gate.

### The rehearsal, which is the deliverable that mattered

`script/netboot-rehearsal` boots nife the way xenon will, on patagonia, with nothing plugged in.
A synthetic ethernet on QEMU's stream `socket` netdev carries real OVMF to the real
`script/board-netboot`, serving the real `target/esp`, with the DHCP answers coming out of the
config file above. No disk of any kind is attached: the wire is the only way in.

Measured, 2026-09-05 on patagonia: `nife x86_64: boot complete, halting.` in 8.2 seconds
from QEMU start, 9,210,880 bytes transferred, blksize 1468 as EDK2 asks for it, the whole
boot tour on serial. A second run an hour later, after the suite had rebuilt the image, measured
7.9 seconds and 9,797,120 bytes, which is the same claim twice and is also the evidence for
the size not being a constant. The negative case (`--mac 02:00:00:00:00:01`) produces `PXE-E16: No valid offer
received` and no boot, which is what a house machine that is not xenon must see.

`-netdev user` was refused rather than missed. QEMU's slirp has a DHCP server and a TFTP server
built in, and pointing them at `target/esp` PXE-boots this image in one line; that was the first
thing tried and it worked, and it is what proved the payload boots at all. It proves nothing about
our configuration or our server, because neither is in the path, and neither can be put there:
slirp's NAT has no TFTP helper, and TFTP moves to a fresh server port after the first packet, so the
DATA reply is dropped by the very NAT that was meant to carry it.

### One defect found in `board-netboot`, and what it cost

EDK2 asks for a file twice: once to learn its size, which it does by starting a read, taking `tsize`
out of the option acknowledgement, and sending a TFTP ERROR to stop the transfer it just
started; and once to fetch it. `board-netboot` did not understand ERROR. It retried its
acknowledgement six times, held the client past its own timeout so the real request had to be sent
again, and printed `FAILED` on the one transfer of a boot that was working perfectly.

Handling it took the rehearsal from 23.4s to 8.2s and made the log true. Milestone 257 wrote
down the shape of this: a network boot can fail in a way that looks like success. This is the same
coin's other face, and it would have been read at the bench as a broken server.

### The bench procedure

`notes/x86-uefi-boot.md`, in the shape `notes/visionfive2.md`'s runbook has: the two firmware
settings with their photograph numbers, the router edit in both UCI and raw-dnsmasq form, the two
commands on patagonia, what the screen and the server terminal should each say in order, and a
failure table keyed on what a person with a camera and no shell can actually see.

It removes the co-location, which is the second reason to want it. First light is on record as
happening the way it did because *"patagonia could not be moved to the bench"*. A netboot does not
need it to be: xenon needs a cable to the house LAN and patagonia needs to be on the same LAN, from
wherever it is.

## The two firmware settings, which are calef's

Read off `notes/xenon-firmware.md`, which transcribed 70 photographs of this machine's setup UI on
2026-09-05 so that a lane would not have to walk to it. Both are on one page:

> IMG_4031, Integrated NIC. `Enable UEFI Network Stack` unticked. Disabled ( );
> Enabled (•); Enabled w/PXE ( ). So the LAN is visible to an OS but there is no UEFI PXE path.

So two changes: tick `Enable UEFI Network Stack`, and move Integrated NIC from `Enabled` to
`Enabled w/PXE`. They are calef's for the reason that note already gives about firmware
generally, that a setting changes this machine's behaviour for everything else it is used for.

Nothing else in the transcription is in the way. Boot List Option is already UEFI, Secure Boot
is Disabled, legacy option ROMs are unticked, and `UEFI Boot Path Security` has no effect
while no admin password is set, and none is.

## The proof that this milestone worked

xenon boots nife with no removable media in it, photographed, since the Dell's video output is
the channel that carried first light. Anything short of that is a rehearsal, and the rehearsal is
now done: it exists, it is green, and it is what makes the bench session worth an evening.

## What this does not fix, and it is the thing that will bite next

xenon halts at POST without a keyboard. `notes/xenon-firmware.md` records `Warnings and Errors`
set to `Prompt on Warnings and Errors`, `Enable Keyboard Error Detection` ticked, and **eight
`Alert! Keyboard not found` entries in the machine's own event log** across a year. A netboot rig
whose point is an unattended power cycle runs straight into that, and the two settings that would
change it are an architect's for the same reason as the two above. **Network boot without that is a
faster bench session, not an unattended one.**

## Status, 2026-10-04 (UTC)

- Firmware half done. calef ticked `Enable UEFI Network Stack` and set Integrated NIC to
  `Enabled w/PXE`. Recorded in the table in `notes/xenon-firmware.md`; photos owed, so no IMG
  numbers are cited yet.
- Router edit remains, and is in calef's homelab agent's hands, via
  `xenon-pxe-spec-2026-10-04.md` (reserve `.138` for patagonia's Wi-Fi MAC, add the four PXE lines).
  Not done as of this record.
- One photographed power cycle remains, with `script/board-netboot` serving, after the router
  edit is confirmed. That is the proof; this milestone is not BUILT until it exists.

## BUGS

- Nothing here has run on xenon, and nothing here can tell you it will. What is proven is OVMF,
  which is EDK2, which is the same codebase Dell's firmware is built from; xenon is at BIOS 1.27.0
  with a Broadcom LOM rather than QEMU's e1000, and the two firmware settings that would make it try
  at all were unticked until 2026-10-04 (see Status below).
- Nothing in this repository has ever talked to the house router. The rehearsal implements what
  the `dnsmasq` lines *mean*; it does not prove `dnsmasq` implements them the same way, and it does
  not prove OpenWRT's UI will accept them. The `dhcp-match` half has no UCI form and has to go
  into `/etc/dnsmasq.conf` directly, which is the part of the runbook most likely to be wrong.
- The `dhcp-boot` server address is patagonia's current lease, and a lease that moves makes xenon
  TFTP into nothing with no console to say so. That happened on 2026-10-04 (UTC): patagonia was
  at `192.168.8.138` on Wi-Fi `en0` (MAC `10:9f:41:d4:8c:0e`), not the `.216` this block recorded, and
  the config is corrected to `.138`. The pin is a router-side reservation for that Wi-Fi MAC (the
  earlier guess, an ethernet MAC, was wrong), requested in a spec for calef's homelab agent and
  not yet confirmed done. Until it is, `.138` is a lease and can move again.
- The transfer rate is a loopback number and is not a LAN number. 9,210,880 bytes in 1.40s
  (6,429 KiB/s) is python talking to itself through a synthetic ethernet with no cable, no switch
  and no loss. radon's real measurement over TFTP on this LAN was 428 KiB/s, and nobody has measured
  a UEFI client on a real wire.
- macOS's firewall is a failure mode this rig has that radon's did not. `board-netboot` is
  python3 binding a UDP port, and the first inbound packet from a machine on the LAN is the first
  time macOS will be asked about it. It is in the runbook's failure table; it has not been hit,
  because the rehearsal is loopback.
- `script/netboot-rehearsal` answers no IPv6 at all. EDK2 tries IPv4 PXE first, so this costs
  nothing today; a firmware that preferred IPv6 would spend its retries in silence before falling
  back, and the rehearsal would look like a timeout with nothing to say about why.
- No fallback, by design, and that is a real difference from milestone 257. radon's card can
  always boot something; a xenon that fails PXE falls to the next boot entry, and the entries on that
  list are `Windows Boot Manager` and whatever removable media is present. That is somebody's
  installed Windows, which argues for leaving a stick in the machine rather than removing it.
- `board-netboot` serves one transfer at a time, and a UEFI client makes two requests per boot
  where U-Boot makes one. The probe is now cancelled promptly rather than held for eighteen seconds,
  so the two no longer collide; two *machines* booting at once still would.

## Follow-on

- **Recorded.** The bench session itself, which is the proof and cannot be a lane: two firmware
  settings, one router edit, one power cycle, and a photograph. The procedure is in
  `notes/x86-uefi-boot.md` and the failure table is written for someone holding a camera rather than
  a shell.
- **Recorded.** The `dhcp-match` lines' OpenWRT siting. The runbook gives two ways in and says
  which is likely to be awkward; whichever one takes should be written back into
  `bench/xenon-netboot/dnsmasq.conf`'s header and into the runbook, because the next person to
  reflash that router will need it.
- **Recorded.** patagonia's ethernet MAC, so the boot server's address can be pinned. One
  `dhcp-host` line, already drafted and commented out, waiting on a value only the bench can supply.
- **Recorded.** The real-wire transfer rate, in this block's BUGS. It is one line of the bench
  session's own output and it is worth writing down next to radon's 428 KiB/s.
- **Recorded.** Three provisional names waiting on an architect, on `script/names --unratified`'s
  worklist rather than blocking anything: `script/netboot-rehearsal`,
  `bench/xenon-netboot/dnsmasq.conf`, and the `bench/xenon-netboot/` directory. The refusals are in
  the script's own `Name:` block, which is where a reader meets them.
- **Recorded.** The keyboard-at-POST hazard, above and in `notes/xenon-firmware.md`. It is calef's
  two settings and it belongs to the unattended-rig work rather than to this one.

## Index row

PXE needs the DHCP server to point at the boot server, which milestone 257 could avoid and this
cannot. The objection died when calef said the router is OpenWRT: not a second DHCP server, one
option on the existing one, scoped to one MAC
