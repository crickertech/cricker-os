---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: 242
decision_dependencies: 46, 135, 86
machine_requirements: x86_64 UEFI silicon with an Intel 8265 wireless card
specific_machine: xenon (the only bench machine with a wireless card, and its room has no Ethernet port)
needs_person: yes
---
# Wi-Fi on a PC that has no Ethernet

Written by `lane/wifi-proposal`, a research lane, on calef's request of 2026-10-04 (UTC). The title,
the slug and every program or crate name below are provisional. No code was written.

**In brief.** Fatal risk 8 (nobody needs it) cannot be tested until a stranger installs nife.
Milestone 494 (a driver for the network card a PC actually has) records in its `BUGS` that a
laptop with no Ethernet port cannot reach rung 3 of milestone 198 (a package manager, and the
trivial install). This proposal prices Wi-Fi for that case, names permissive code for every layer,
and weighs it against two cheaper ways onto a network. The recommendation is that USB tethering
to a phone comes first and Wi-Fi second. Tethering lets the Wi-Fi firmware ship as a package
rather than in the base image, which turns the hardest decision here into a reversible one.

## What xenon's card is, read from the machine

xenon's PCI survey, captured on 2026-10-04 (`bench/xenon-2026-10-04/install-and-disk-boot.log` on
branch `lane/xenon-install-record`, lines 107 and 322), prints `02:00.0 8086:24fd class 028000`.
Device `24fd` is the Intel Wireless 8265 (OpenBSD `pcidevs`: `WL_8265_1 0x24fd`). The same id is
also used by the 8275, and the survey does not print the subsystem id that tells them apart. Both
take the same driver and firmware, so nothing below depends on it. The card sits behind root port
`00:1c.0`, so it falls to the catch-all VT-d unit at `0xfed91000`, which the same boot reports as
translating.

The firmware is in linux-firmware as `intel/iwlwifi/iwlwifi-8265-36.ucode`, 2,432,528 bytes
(`-34` is 2,440,780). OpenBSD loads the same image as `iwm-8265-36`. Its licence,
`LICENSES/LICENCE.iwlwifi_firmware`, says three things that matter here:

- Redistribution in binary form is permitted "without modification", keeping Intel's notice.
- "No reverse engineering, decompilation, or disassembly of this software is permitted."
- The patent licence applies only when used "alone, or in combination with an operating system
  licensed under an approved Open Source license". nife's MIT OR Apache-2.0 qualifies.

So the blob may be shipped and may not be inspected.

## Loading it fits the confined-driver model, and makes risk 7 sharper

The driver host loads firmware by DMA. The process stages the image's sections in a DMA buffer and
points the card's service channel at them. Nothing needs a privileged path. The shape is that
of milestone 261 (the NVMe driver leaves the kernel), which is DECISIONS §86 (whether an NVMe
driver can leave the kernel) option 2a. Milestone 494 applied it to the I219. The kernel
enables the function, confines its DMA with `iommu::confine`, and grants the process a window of
BAR0 and a DMA region. Rings, the command queue and firmware staging all live in that region. On
494's I219 the ring base and tail registers share a page, so the IOMMU is the whole of the DMA
confinement. The 8265 is the same case, from OpenBSD's `if_iwm.c`, where reset and queue
registers sit in one BAR.

That matters more here than for a NIC. Once alive, the firmware is 2.4 MB of code nobody here may
read, running on the card's own processor as a bus master, parsing frames that anyone in radio
range can send. The IOMMU is the only thing between it and memory. A Wi-Fi card is therefore the
strongest live demonstration of fatal risk 7 (the confinement claim is false) the bench could
offer, and its failure would be equally loud. Milestone 556 (a second RISC-V implementation, for €16 a
month) says a target needing "a firmware blob this tree cannot inspect" stops being a nife target.
That sentence is about boot firmware. A peripheral's firmware held behind an IOMMU is the case the
confinement claim exists for, and the distinction should be said where the blob ships.

## The pieces, and permissive code for each

Every row was read from its source on 2026-10-04 (UTC). Line counts are `wc -l` of current trees.
Fuchsia counts are non-test source files, but its Rust files carry inline tests, so they overstate.

| piece | candidate | licence | size | covers 8265 | portability to a nife program |
|---|---|---|---|---|---|
| device driver | OpenBSD `sys/dev/pci/if_iwm.c` | ISC, plus Intel's dual BSD/GPLv2 block | 12,243 lines of C | yes | translate to Rust; no C reaches a nife program today (milestone 442 (a crypto provider `rustls` can use on all three bare-metal targets) found C providers do not build) |
| device driver | FreeBSD `sys/dev/iwm/if_iwm.c` | ISC plus Intel dual | 6,643 lines of C | yes | same; less maintained than OpenBSD's |
| device driver | Fuchsia `drivers/third_party/intel/iwlwifi` | BSD-3-Clause only (Fuchsia kept the BSD half) | 118,567 lines of C/C++ | yes, lists `0x24FD` | every Intel generation plus a test mode; ten times iwm for the same chip |
| 802.11 station | OpenBSD `sys/net80211` | BSD-3-Clause and ISC | 24,693 lines of C, 31 files | n/a | a station needs perhaps a third; written for a kernel with timeouts and interrupts |
| 802.11 station | Fuchsia `wlan/lib/mlme` and `lib/sme` | BSD-3-Clause | 28,100 and 19,310 lines of Rust | n/a | Rust, but 32 of 33 `mlme` files use Fuchsia's FIDL or `zx` interfaces (2,876 lines) |
| frames | crates.io `ieee80211` 0.5.9 | MIT OR Apache-2.0 | small | n/a | builds without `alloc`; parses frames and RSN elements, no handshake |
| WPA2 supplicant | Fuchsia `wlan/lib/rsn` | BSD-3-Clause | 10,268 lines of Rust | n/a | 17 lines touch Fuchsia interfaces; 12 files reach BoringSSL through `mundane`, `bssl-sys` or `fcg-crypto` |
| WPA2 supplicant | OpenBSD `ieee80211_pae_input.c`, `_pae_output.c`, `_crypto.c` | ISC | 2,520 lines of C | n/a | WPA2-PSK in the kernel with no `wpa_supplicant`; no WPA3 |
| WPA3 (SAE) | Fuchsia `wlan/lib/fcg-crypto` | BSD-3-Clause | about 4,300 lines | n/a | BoringSSL underneath; `p256` is already in the tree |
| supplicant | `wpa_supplicant` 2.12 | BSD-3-Clause (advertising clause removed) | about 630,000 lines | n/a | refused: a POSIX daemon over nl80211, larger than this whole tree's networking |
| crypto | RustCrypto | MIT OR Apache-2.0 | n/a | n/a | `hmac`, `aes`, `sha2`, `p256` are in `cryptography_provider` (milestone 442); WPA2 adds `sha1`, `pbkdf2` and `aes-kw` |

Two prior-art facts change the question.

- The premise that "iwlwifi is GPL" is half wrong. Linux's iwlwifi files carry `GPL-2.0 OR
  BSD-3-Clause` (read in `mvm/mac80211.c`, `pcie/gen1_2/trans.c`, `iwl-drv.c` and others), and
  OpenBSD's iwm says it is "based on BSD-licensed source modules in the Linux iwlwifi driver". What
  is GPL-only is mac80211 (`net/mac80211/main.c`: `GPL-2.0-only`), the Linux 802.11 layer every
  Linux driver sits on. The driver is takeable; Linux's stack around it is not.
- Fuchsia is the only fully permissive stack found that names this chip, and most of it is Rust.

### Rust operating systems

Redox has no wireless driver (`redox-os/drivers/net` holds `alxd`, `e1000d`, `ixgbed`, `rtl8139d`
and `virtio-netd`). No Wi-Fi was found for Theseus, Tock or Hubris; that is not found rather than
proven absent. Embassy's `cyw43` runs the WPA handshake inside the chip's firmware, which the 8265
does not offer. `supplicant-rs` (Apache-2.0) does the 4-way handshake and SAE, but over Linux
nl80211 and tokio.

### Driver-compatibility layers, and why each lands in a package

| layer | what it runs | licence of the result | verdict |
|---|---|---|---|
| Genode `dde_linux`, `pc_wifi` | Linux 6.18.19's iwlwifi, mac80211 and (implied by its config format) `wpa_supplicant` | GPLv2 through mac80211; Genode itself is AGPLv3 | package only under §135, so useless at install time |
| TU Dresden DDE/DDEKit | Linux 2.6 drivers | GPLv2 for the Linux part (from a search snippet, not read) | stale, and GPL |
| NetBSD rump kernels | `libnet80211` and only `iwn` among wireless drivers; no rump `iwm` | BSD (not read) | NetBSD's `iwm` does 802.11a/b/g only |
| FreeBSD LinuxKPI `iwlwifi` | Linux iwlwifi under a shim over net80211 | the BSD half of iwlwifi | lists the 8265, but 802.11n/ac only on 22000 and later |

A compatibility layer is the fastest way to a working Wi-Fi package and the one route that cannot
serve the base. Genode tracks recent Linux and would bring every Intel generation at once, which
is a real argument for a later GPL package. It is not an argument for install time.

## Where it lives, and what that excludes

If Wi-Fi is the only way a laptop reaches the network during install, the driver is base. The base
carries no copyleft (§135 (running GPL software is aggregation), unchanged by its 2026-09-27 amendment). That excludes mac80211, Genode's
layer, DDE and any GPL-only driver. It does not exclude iwlwifi's BSD half or anything in the
table above.

It also raises a question §135 does not answer. The firmware is neither copyleft nor permissive.
It is proprietary and redistributable. No decision in this tree says whether the base image may
carry such a file, and that is a fact that leaves the machine. Debian (since 12) and OpenBSD
answer it in opposite ways: Debian puts non-free firmware on its install media, and OpenBSD's
`fw_update` fetches it after install (both from memory, not read today). The OpenBSD answer
needs a network before Wi-Fi works, which is exactly what tethering supplies. That is the case for
ordering below.

## Three ways onto a network for a laptop with no Ethernet

| | Wi-Fi (8265) | USB tethering to a phone | USB-Ethernet dongle |
|---|---|---|---|
| what the stranger needs | nothing extra | a phone and its cable | a dongle |
| firmware blob | 2.4 MB, unreadable | none | none |
| testable under QEMU | protocol layers only; no 802.11 device exists in QEMU | yes: QEMU's `usb-net` emulates CDC Ethernet and RNDIS | CDC-ECM yes; vendor chips no |
| covers | Intel 7260 to 9260 via iwm; not AX200 and later (OpenBSD `iwx`) | any machine with a USB port | class-compliant dongles; ASIX and Realtek need vendor drivers |
| retention (does a user keep it two months) | yes | poor: nobody keeps a phone tethered | moderate |
| permissive sources | iwm, net80211, Fuchsia `rsn` | OpenBSD `if_cdce.c` 859 lines, `if_urndis.c` 1,523, FreeBSD `if_ipheth.c` 662 | `if_cdce.c`; OpenBSD's Realtek and ASIX drivers, 2,642 and 1,511 |
| lines nife writes, estimated | 12,000 to 16,000 | 3,000 to 4,000 | 1,500 for ECM alone |

Android phones tether over RNDIS on older models and CDC-NCM on recent Pixels (Gentoo's tested
list; the Android version that switched was not found). iPhones use Apple's `ipheth` protocol.
FreeBSD's `if_cdce.c` is the one NCM driver found, and it is BSD-4-Clause, which `deny.toml`'s
allow-list would refuse, so NCM is written from the USB-IF specification.

All USB rows ride on milestone 242 (USB host and HID), in pull request #1629, whose xHCI does control
and interrupt transfers but not bulk ("No streams, no isochronous, no bulk"). Its Normal TRB
builder already exists. A USB network device does no DMA of its own: the xHCI does it, inside the
domain 242 already confines, so no new bus master is added.

The surprise worth stating: xenon is itself the stranger's case. Milestone 494's bench notes (`notes/e1000e.md` on its branch) say
xenon's room has no Ethernet port (calef, 2026-10-04). Wi-Fi is the one network xenon can reach
where it sits, and tethering is the other.

## The sequence

Each step ships alone and has one checkable exit. Estimates are by analogy. Milestone 494 wrote
1,934 lines (its crate, kernel control plane, spawn and transport) for one NIC. A translated C
driver is guessed to land near half its source length in Rust when one chip generation is kept,
and that guess is recorded as one.

### Tethering first

| step | ships | exit | lines | where it is tested |
|---|---|---|---|---|
| T1 | bulk endpoints in 242's xHCI; a CDC-ECM class driver as a `phy::Device` | `net_stack` gets a DHCP lease through `-device usb-net` under QEMU on all three architectures | 1,200 to 1,800 | QEMU |
| T2 | RNDIS, adapted from OpenBSD `if_urndis.c` (ISC) | the same lease through `usb-net`'s RNDIS configuration | 700 to 900 | QEMU |
| T3 | CDC-NCM, written from the USB-IF specification | a lease from a recent Android phone on xenon | 700 to 900 | xenon and a phone |
| T4 | `ipheth`, adapted from FreeBSD `if_ipheth.c` (BSD-2-Clause) | a lease from an iPhone on xenon | 400 to 600 | xenon and an iPhone |
| T5 | rung 3c over a phone | from xenon's installed system, a package fetched by host name over tethering | under 300 | xenon |

### Wi-Fi second

| step | ships | exit | lines | where it is tested |
|---|---|---|---|---|
| W0 | the firmware ruling (below); no code | a section in `design/decisions/` | 0 | n/a |
| W1 | the card enumerates and its firmware loads under confinement: kernel control plane, TLV parser, section upload, polled ALIVE | xenon prints the firmware's version from its ALIVE response, with the `vt-d` owner line; host tests parse the real `.ucode` | 3,000 to 4,000 | xenon; host |
| W2 | NVM read (MAC address, regulatory), PHY calibration, scan | xenon lists the SSIDs a phone beside it lists | 2,000 to 3,000 | xenon |
| W3 | open association, 802.11 to 802.3 conversion, data queues, DHCP through `net_stack` | a lease from an open network (a phone hotspot set open) | 2,500 to 3,500 | xenon |
| W4 | WPA2-PSK: Fuchsia `rsn` adapted to RustCrypto, keys installed in firmware for hardware CCMP | a lease from the house network; host test of `rsn`'s supplicant against its own authenticator, plus IEEE 802.11 Annex J vectors | 1,500 to 2,500 | host; xenon |
| W5 | rung 3c over Wi-Fi, with SSID and passphrase asked for at the installer prompt | a package fetched by host name over Wi-Fi from xenon's installed system | 500 to 1,000 | xenon |
| later | WPA3-SAE and 802.11w (Fuchsia `fcg-crypto` over `p256`); `iwx` for AX200 and later | a stranger's newer laptop | not priced | |

W1 to W4 each need a xenon boot, so each costs calef's attention as well as a lane. That is the
cost tethering avoids for T1 and T2.

What QEMU cannot test: no QEMU device speaks 802.11 (its device list has none). The Android
Automotive reference runs Linux's `mac80211_hwsim` behind `virtio-net` for this, and hwsim is
GPLv2 and Linux-only. The honest split is that the protocol layers (frames, MLME state machine,
`rsn`) are host-tested and the device half is xenon-only, as 494's I219 bring-up is. A host-side
model of the 8265's command interface, like 494's simulated device, would prove the driver against
a model, not against the card. Passing a USB Wi-Fi adapter through QEMU on patagonia (macOS) was
not tried and is not recommended as a gate.

## Forks for an architect

- F1, the firmware (irreversible: a file that leaves the machine). Options: carry the blob in the
  base image (Debian); ship it as an opt-in package fetched after install (OpenBSD); ship it on
  the install stick only. Recommendation: a package, which tethering makes sufficient for install.
  Revisit if the stranger test shows tethering is not available to strangers.
- F2, where the Wi-Fi stack runs (a frame protocol two programs agree on). Inside `net_stack`, as
  494's driver does, or its own process handing Ethernet frames to `net_stack`. At equal cost the
  separate process wins, since it keeps the radio-facing parser and the firmware's bus away from
  the TCP stack. Recommendation: its own process, and that is not an effort argument. 494 chose
  the other way for effort, and said so.
- F3, dependencies: `sha1`, `pbkdf2` and `aes-kw` for WPA2 (RustCrypto, same family as the
  provider), and `ieee80211`. Each is a §46 (thin primitives or whole subsystems) ruling.

## The seven questions

1. Alternatives. Wi-Fi first loses on testability and on the blob ruling it forces before install.
   A dongle loses on what the stranger must buy. Compatibility layers lose on licence for base.
2. The tree's analogue. Milestone 494: take FreeBSD's field knowledge, write the confinement split.
   Milestone 242 for USB. §135 for what base may carry.
3. Prior art. The tables above, read today; the Debian and OpenBSD firmware policies are recalled.
4. The premise. "iwlwifi is GPL" is false for the driver and true for mac80211. "Base, so
   permissive" holds, but it does not decide the blob, which is neither. "Wi-Fi is needed at
   install" holds only if no other network exists, and tethering supplies one.
5. Cost. Tethering T1 to T5 about 3,000 to 4,500 lines, two of five steps on QEMU alone. Wi-Fi W1
   to W5 about 9,500 to 14,000 lines, every step on xenon. These are estimates by analogy, not
   measurements.
6. Reversibility. The code is reversible. F1 and F2 are not, and nobody has acted on either yet.
7. Equal cost. If both cost the same, tethering would still come first for install, because it is
   testable in QEMU and needs no blob. Wi-Fi would still be needed for retention, which is risk 8's
   actual claim. So the order is not about effort.

## Recommendation

Raise T1 to T5 as a milestone after 242 lands, and Wi-Fi W0 to W5 as a second milestone with W0
ruled first. Take the driver from OpenBSD's `if_iwm.c` and the supplicant from Fuchsia's `rsn`.
Write the confinement split and the station state machine here, using OpenBSD's net80211 and
Fuchsia's `mlme` as references. Before W1, decide whether the first Wi-Fi target is xenon's 8265
(the bench machine) or `iwx`'s AX200 family (what a 2026 stranger's laptop more likely has,
recalled). The question is the one 494 asked about NICs, and xenon is why the 8265 is recommended.

## Reuse

Taken or adapted:

- OpenBSD `if_iwm.c` (ISC with Intel's dual BSD/GPLv2 block) for the 8265 driver.
- Fuchsia `wlan/lib/rsn` (BSD-3-Clause) for WPA2, with its BoringSSL calls replaced by RustCrypto.
- crates.io `ieee80211` (MIT OR Apache-2.0) for frames.
- OpenBSD `if_urndis.c` (ISC) and FreeBSD `if_ipheth.c` (BSD-2-Clause) for tethering.

Written, with reasons:

- The confinement split, because no candidate has one (as in 494).
- The station state machine. Fuchsia's `mlme` uses its own OS interfaces in 32 of 33 files, and
  OpenBSD's net80211 is about three times what a station needs.
- CDC-NCM, because the one driver found is BSD-4-Clause.

Refused: `wpa_supplicant` (about 630,000
lines over nl80211), Fuchsia's `iwlwifi` (118,567 lines against iwm's 12,243 for the same chip),
and every GPL layer for base.

## BUGS

- The 8265 and the 8275 share `24fd`; the subsystem id is unread. It does not change the driver.
- Whether iwm offloads group-key CCMP to hardware is unverified. If not, W4 adds software CCMP
  through RustCrypto's `ccm`.
- All line estimates are analogies to 494, not measurements.
- The Android version that moved tethering from RNDIS to NCM, and whether an iPhone tethers over
  NCM, were not found.
- Fuchsia's line counts include inline tests.
