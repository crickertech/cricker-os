---
status: BUILT
raised: 2026-09-04
built: 2026-09-05
---
# 257. Boot radon over the network, so the microSD card stops being the tax on every experiment

Minted 2026-09-04 by calef, the same evening the path was proved by hand at
radon's U-Boot prompt; built 2026-09-05. *(Number provisional until the merge queue lands it.)*

## The card is the bottleneck, and the tree predicted the day this would matter

`notes/visionfive2.md` has carried this since 2026-09-01, under **TFTP alternative (not built)**:

> the cordoba side (dnsmasq or tftpd serving the image) is its own small piece of work and turns the
> flash-a-card loop into a rebuild-and-reset loop. **Worth building the moment the card loop gets
> annoying, which history says is the second bench session.**

The 2026-09-04 E3 session wrote the card six times, once per boot, each a walk to the board and
back, because E3 is a comparison of two builds and the interleaving that makes it valid requires
alternating them. That session also produced the other half of the argument: the numbers it took
were good, so the whole cost was handling.

**One correction the note carries and this block does not inherit: the server belongs on patagonia,
not cordoba.** radon's UART goes into patagonia (corrected by calef 2026-09-04; the rig note said
cordoba and was wrong), and patagonia is where the images are built. Serving from patagonia means
there is no copy step at all: build, power cycle, watch, on one machine.

## What was proved by hand, 2026-09-04, and what it cost

Driven from patagonia over the serial line, at the board's `StarFive #` prompt. The one real
unknown, whether the vendor firmware has a network stack, was checked rather than assumed:

```
U-Boot 2021.10 (Feb 12 2023 - 18:15:33 +0800), Build: jenkins-VF2_515_Branch_SDK_Release-24
dhcp     - boot image via network using DHCP/TFTP protocol
tftpboot - boot image via network using TFTP protocol
ethernet@16030000 Waiting for PHY auto negotiation to complete...... done
DHCP client bound to address 192.168.8.200 (3378 ms)
```

Then nife booted entirely over the wire, card ignored, and the full bench suite ran to `bench: done`:

```
tftpboot ${kernel_addr_r} nife-vf2.img        282,624 bytes,   1.4 s
tftpboot 0x90000000 nife-initrd.img         9,044,480 bytes,  20.6 s  (428 KiB/s)
setenv nife_archive_size ${filesize}
fdt addr ${fdtcontroladdr}
fdt move ${fdtcontroladdr} 0x86000000
booti ${kernel_addr_r} 0x90000000:${nife_archive_size} 0x86000000
```

Only the two `load` lines of `target/board/boot.cmd` changed. `fdt move` and `booti` are byte for
byte what milestone 218's script already emits, which is why this is a small change rather than a
new boot path.

### It also produced a control nobody asked for, and it is the reason to trust the workflow

The image served was the **padded** E3 build, so that boot is a fourth reading of the padded
condition, taken through a completely different load path: DHCP, ARP, and about 6,200 TFTP round
trips instead of a FAT read.

| row | card-booted, 3 boots | TFTP-booted |
|---|---|---|
| `ipc_rtt` | 4311 · 4310 · 4310 | **4311** |
| `call_reply` | 5088 · 5089 · 5088 | **5089** |
| `ipc_rtt_el0` | 124958 · 124391 · 124903 | **124917** |

All three land inside the card-booted cluster. **How the kernel arrives does not perturb what it
measures**, the one thing that could have made this workflow useless for bench work.

## What was built, 2026-09-05

Three things, and the second is the one that matters.

`cargo xtask board-script --tftp`, which emits `dhcp` plus two `tftpboot` lines where the card
script has two `load` lines. `script/board-image --tftp` passes it through. The default is
unchanged and stays unchanged: `script/board-image` and `script/board-image --card` write the
same card-only script milestone 218 shipped, byte for byte, and a test asserts the constant is
untouched. A network-booting card is a promise about a machine that has to be running, so it is
asked for rather than arrived at.

The card underneath the network, exercised rather than asserted. The script tries
DHCP and two transfers and falls back to `load` when any of them fails, so a card written once can
be left in radon and still boots with the cable out. Its shape is deliberately the dullest thing
that can express a fallback: one state variable, `if cmd; then` and `fi`, nesting never deeper than
two, no `else` anywhere, and `netretry no` first so an absent network fails in seconds instead of
retrying at an empty bench.

`if cmd; then ... fi`, `test x${v} = xy` and `${v}` expansion are common to U-Boot's hush and POSIX
`sh`, so the generated script is run under `/bin/sh` in a test with the six U-Boot verbs stubbed as
shell functions whose exit status the test picks. Four cases: everything works and the card is
never touched; no lease, so the card supplies both halves; the kernel transfer succeeds and the
archive's fails, which must still take both halves from the card because a mixed pair halts at
`MEASURED BOOT REFUSED`; and neither path having a payload, where nothing is booted and the board
says so rather than jumping into whatever a previous boot left at `0x4020_0000`. That last case is a
guard the manual sequence lacks.

`script/board-netboot`, a read-only TFTP server with `blksize` and `tsize`, in python3. The
decision against dnsmasq is recorded in the script's header, and the deciding argument is not the
dependency one: dnsmasq is a DHCP server that also does TFTP,
this bench LAN is a family's house network with a router already handing out leases, and a second
DHCP server on it is an outage for everyone in the building. A tool that cannot speak DHCP cannot
get that wrong. python3 is already what ten `script/` entry points are written in, so it asks
nothing new of anybody's machine.

### The address expectation, which is now not an expectation

There is no server address constant anywhere in this tree. `cargo xtask board-script --tftp`
reads the address off the machine writing the card, at the moment it writes it, and the script
echoes it at boot so a console log says what a card expects before anything depends on it. A moved
lease is then one line at the prompt (`setenv nife_boot_server <addr>`, `source ${scriptaddr}`) rather
than a card reader.

The first implementation was wrong and the machine said so, the lesson of milestone 256 (PCI BARs in a
hardcoded window) arriving in a new place. The obvious discovery is a
connected UDP socket whose local address the kernel picks from the route. patagonia's default route
belongs to a Tailscale interface, so every probe answered `100.75.22.70`, a CGNAT address radon has
no path to, and a card written that evening would have silently fallen back to the card forever.
Interfaces are enumerated instead and anything outside RFC 1918 is dropped. patagonia turns out to
have two addresses on the bench LAN, `en0` at `.216` and a USB adapter at `.206`; the first is
taken, both are printed, and `--server` picks the other. `.216` is what the 2026-09-04 boot used, so
the discovery reproduces the proved value without anyone having written it down.

## What was to be built

1. A `--tftp` mode for `cargo xtask board-script` (`dhcp` and two `tftpboot` lines for the two
   `load` lines), with the matching `script/board-image` flag.
2. A fallback to the card in the same script, since U-Boot can branch on command status. That lets
   the card be written once and left in radon: no cable, no hub, no DHCP, and the board still boots.
   A card bricked by an unplugged cable is a worse rig than the one being replaced.
3. A TFTP server on patagonia. A read-only server with `blksize` (RFC 2348) is about a hundred
   lines and was written for the 2026-09-04 session; a `script/` entry point or a Homebrew `dnsmasq`
   in TFTP-only mode is the lane's call. Port 69 bound without root here, so neither needs `sudo`.
4. A recorded address expectation. `serverip` is patagonia's, `192.168.8.216` on 2026-09-04, and
   a DHCP lease can move it. The script should say so where a reader meets it rather than leave a
   stale constant for the bench, the defect class milestone 256 is about, one layer out.

## The measure

**A bench session that never touches the card.** Concretely: write the card once with the new
script, then take two boots of different builds with no physical access between them.

## What this does not do

**It is not remote power.** A hung board still needs a person, because radon's software reset path is
closed: `notes/soak.md` records SBI SRST resetting the SoC and U-Boot SPL then failing to re-init the
PMIC, so the board does not come back. calef accepted manual power on 2026-09-04; milestone 224
holds that decision and the options for revisiting it.

**And it is not unattended CI.** Three things stand between here and that, and this is one:
network boot (this block), an on-board test-suite exit so a machine rather than a person reads the
result (milestone 16's remaining piece), and remote power (224, accepted as manual). Two of the
three are lanes; the third is a decision that has been made the other way, on purpose.

## Used in anger, 2026-09-16: seven boots, no card touched

A five-boot job-mix session plus a bench boot and the tour, all fetched over the wire from an
identical image, with the microSD card written once at the start of the day and never removed
again. Transcripts under `bench/radon-2026-09-16/`.

```
ethernet@16030000 Waiting for PHY auto negotiation to complete...... done
DHCP client bound to address 192.168.8.200 (257 ms)
TFTP from server 192.168.8.206; our IP address is 192.168.8.200
nife: payload came from net
```

Kernel at 3.2 MiB/s, the 8.6 MB archive at 6.7 MiB/s, against the six card writes of 2026-09-04.

The fallback was exercised too, by accident, and that is the more useful half. The first boot of
2026-09-16 came up before the ethernet cable was in, and printed
`Waiting for PHY auto negotiation to complete......... TIMEOUT !` followed by
`nife: payload came from card`. The board booted anyway, off the card, and said which path it took:
the first evidence that a network boot degrades rather than strands the board. Recorded in
milestone 218's block as well, where the hands-free claim it supports lives.

## Confirmed on radon, 2026-09-05, and it found a bug no test could have

Three boots, with the card written by `--tftp` and left in the board throughout.

Boot 1, ethernet unplugged. The fallback works and hush accepts the script, this block's headline
risk, since the script had only run under `/bin/sh` with the U-Boot verbs stubbed:

```
nife: tftp server is 192.168.8.216, setenv nife_boot_server to point somewhere else
ethernet@16030000 Waiting for PHY auto negotiation to complete......... TIMEOUT !
ethernet@16040000 Waiting for PHY auto negotiation to complete......... TIMEOUT !
nife: nothing came over the network, falling back to the card
nife: payload came from card
```

The whole capture, two PHY timeouts plus a complete boot tour, fit in under 25 seconds. A card
left in radon with no network is not a session-killer.

Two claims in this block's BUGS are answered by that boot, and one is corrected. U-Boot tries
both of radon's ports, `16030000` then `16040000`, so the cost of no network is two PHY
timeouts rather than one. And **`netretry no` was not what bounded it**: PHY auto-negotiation times
out first, in the ethernet driver, before `dhcp` gets far enough for retry logic to matter. The
practical answer is unchanged and the mechanism named in this block was the wrong one.

Boot 2, ethernet plugged in, `board-netboot` serving. It fell back to the card anyway, and the
reason is a real defect:

```
DHCP client bound to address 192.168.8.200 (257 ms)
TFTP from server 192.168.8.1; our IP address is 192.168.8.200
TFTP server died; starting again
nife: nothing came over the network, falling back to the card
```

**Bare `dhcp` in U-Boot means "get an address AND TFTP the bootfile from the server DHCP names",
not "get an address".** So the board fetched from `192.168.8.1`, the house router, failed, and
because `dhcp` returns failure when its own autoload fails, the entire network branch was skipped.
It looked like a clean fallback and was a bug, the worst shape a defect can take in a mechanism
whose job is to fall back quietly.

Boot 3 confirmed the one-line fix without rewriting the card. Autoboot was interrupted,
`setenv autoload no` typed by hand, and the card's existing script sourced:

```
TFTP from server 192.168.8.216; our IP address is 192.168.8.200
Bytes transferred = 282624
Bytes transferred = 9051648
nife: payload came from net
```

`setenv autoload no` is now emitted by `board_network_boot_script`, with the reason at the call
site.

Why no test could have caught it: every test of this script stubs `dhcp` and chooses its exit
status, so none can observe what the real command *does* on success. The line was in the manual sequence that proved this path on 2026-09-04 and was left out of
the transcript in this block that the script was written from. The record was the defect, and
the lane built correctly from an incomplete one.

Transcripts: `bench/radon-2026-09-05/`.

## BUGS

- None of this has run on radon, and that is the honest headline. The board needs calef at the
  bench and the lane did not have him. What the 2026-09-04 evening proved is the *sequence*, typed
  by hand; what is unproved is that U-Boot's hush accepts this *file*. `sh` is not hush, so the
  fallback test above proves the control flow and says nothing about the parser. The specific
  untested claims: that hush takes `if cmd; then` on one line, `test x${v} = xy`, `!=`, and `if`
  nested one deep inside another; that `dhcp` and `tftpboot` return a failing status rather than
  halting the script; and that `netretry no` makes a dead network fail in seconds. Each is
  documented U-Boot behavior and none of it has been watched here.
- A misjudged `netretry` is the one that would hurt. If a network that is not there hangs `dhcp`
  instead of failing it, a card left in the board waits at an empty bench rather than falling back,
  which is the exact failure this milestone was written to make impossible. It is the first thing to
  check on the next bench session, by unplugging the cable and powering the board.
- 428 KiB/s is slow, and the 9 MB archive is 20 seconds of it. That is fine against a two-minute
  walk and it is worse than a card read, so a session that boots many times pays it many times.
  Nobody has looked at whether U-Boot's `blksize` or the TFTP window is the limit. `script/board-netboot`
  accepts a block size up to 9000 and will honor whatever U-Boot asks for.
- The card still has to be written once, and a change to the boot script means writing it again.
  The loop is shorter, not gone.
- `ethernet@16030000` is one of radon's two ports, and it is the one that was plugged in. Nothing
  here says what happens on the other, or whether U-Boot would find it. A `--tftp` card on the wrong
  port falls back to the card rather than failing, which makes this quieter than it was: read the
  `payload came from` line rather than assuming.
- `script/board-netboot` serves one transfer at a time and trusts the LAN. Any host that can reach
  udp/69 can read any file under `target/board`. There is one board, and the directory holds a
  kernel and an archive that are about to be published anyway.
- **The measure is not met yet.** "A bench session that never touches the card" needs a bench
  session, and the next one is where this is either true or a bug report.

## Follow-on

- **Recorded.** The bench confirmation. Everything here is unexercised on radon, and this block's
  BUGS section names which claims specifically. It is not a milestone: it is the first ten minutes of the
  next bench session, and the triage row in `notes/visionfive2.md` is what to read.
- **Done.** Both names are ratified by calef, 2026-09-05. The lane shipped `board-tftp` and
  `nife_server`; the answers are `script/board-netboot` and `nife_boot_server`, and the two
  corrections are the same one applied at two levels. `board-netboot` beat `board-tftp` because the
  `board-*` family names the thing rather than the mechanism (`board-image` names the image,
  `board-console` names the console, and `script/server`'s header says outright that "an OS is what
  you start"), so naming the wire would have made this the one member describing its transport.
  `nife_boot_server` beat both `nife_server` and `nife_tftp_server` for the same reason one level
  down: the protocol is not what the address is for.

  `nife_boot_server` also earns its extra word where the name actually lives. The variable
  outlives the script, in the U-Boot environment, where a person meets it at `printenv`; there
  `nife_server` reads as "nife's server, which one?". The provenance is in
  `script/board-netboot`'s header, which carries the refusals, and the boot script prints the
  variable at every boot. It is a U-Boot environment variable rather than a crate or a program, so it
  falls outside the naming rule's stated scope, and it is still a name a person types at a prompt.
- **Recorded.** `blksize` and the 428 KiB/s, in this block's BUGS. Nobody has measured whether the
  limit is the option, the window, or the board, and 20 seconds against a two-minute walk does not
  buy an investigation yet.
- **Milestone 224.** Remote power, and it stays there: manual power was accepted on 2026-09-04 and
  nothing here changes that argument.

## Index row

`script/board-image --tftp` writes a boot script that fetches over the network and falls back to
the card, `script/board-netboot` serves it, and no server address is written down anywhere: it is
read off the machine writing the card and echoed at boot. The fallback is exercised under `/bin/sh`; none of it has run on radon
