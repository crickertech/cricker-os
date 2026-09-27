# xenon's firmware, page by page

*Name provisional. calef names the interfaces (§75), and this note's own filename is one of them;
`xenon-firmware.md` is what a lane picked, and it is expected to change.*

**Dell OptiPlex 7050 Micro, BIOS revision 1.27.0**, Service Tag `25XNBM2`, manufactured
12/22/2017. Every setting below was read off a photograph of the machine's own setup UI, taken by
calef on **2026-09-04** during milestone 87's first light (`notes/x86-uefi-boot.md`).

**Provenance, and it is the unreproducible kind.** The originals are 70 `.HEIC` files in
`~/projects/xenon/` on patagonia, calef's machine, named `..._o_IMG_40NN.HEIC`. They are **not in
this repository** and are not going to be: 120 MB of a format no browser renders, recording a state
that a single visit to the setup screen can change. This note is the record; the photographs are
the negatives. Each entry below carries its `IMG_` number so a reader with the originals can go
back to the frame.

**A firmware record is a record of one moment.** Anybody who changes a setting at the bench owes
this file an edit, because the next lane will read it instead of walking to the machine.

## What nife depends on

The short list, first, because these are the ones a bench session or a lane actually looks up.

| Setting | Value | Where | Why nife cares |
|---|---|---|---|
| Boot List Option | **UEFI** | IMG_4027 | The stick is `\EFI\BOOT\BOOTX64.EFI`; Legacy would look for an MBR boot sector that is not there |
| Secure Boot Enable | **Disabled** | IMG_4058 | The image is unsigned and nothing in this tree signs it |
| Enable Legacy Option ROMs | **unticked** | IMG_4028 | Off is what UEFI boot mode wants, and Secure Boot forbids it being on |
| Serial Port | **COM1** (`3F8h`, IRQ 4) | IMG_4032 | Exactly where `arch::x86_64::port` and `drivers/ns16550.rs` look |
| Enable USB Boot Support | **ticked** | IMG_4036 | Without it the stick is not a boot device |
| **Enable VT for Direct I/O** | **ticked** | IMG_4084 | VT-d is **on**. See below |
| Enable Intel Virtualization Technology | **ticked** | IMG_4083 | |
| SATA Operation | **AHCI** | IMG_4033 | Not RAID, so the NVMe is a plain PCIe function rather than hidden behind Intel RST |
| M.2 PCIe SSD-0 | **enabled**, `Micron 2450 NVMe 256GB`, 256 GB | IMG_4034 | The NVMe DECISIONS §86 wants an IOMMU in front of |
| SATA-0 / SATA-4 | enabled, both `(none)` | IMG_4034, IMG_4026 | Nothing is attached; the only storage is the M.2 |
| Multi Core Support | **All** (4 cores) | IMG_4062 | The boot tour's "4 cores enumerated, 4 enabled" is a setting, not a coincidence |
| Memory Installed | **16384 MB** (2x8192 MB DDR4-2400, dual channel) | IMG_4025 | See the discrepancy note below |
| ASPM | **Disabled** | IMG_4094 | PCIe links stay out of low-power states, which is one fewer variable for a first driver |
| TPM 2.0 Security | **TPM On**, Enabled, SHA-256 | IMG_4050 | `measured_boot` has a TPM here if it ever wants one |
| Wake on LAN/WLAN | **Disabled** | IMG_4071 | Nothing can wake this machine over the network |
| AC Recovery | **Last Power State** | IMG_4067 | A smart plug cycling the mains returns the machine to whatever it was |
| Warnings and Errors | **Prompt on Warnings and Errors** | IMG_4080 | This machine stops at POST and waits for a keypress. See the hazard below |
| Enable Keyboard Error Detection | **ticked** | IMG_4076 | A missing keyboard is one of the things it stops for |
| Fastboot | **Thorough** | IMG_4077 | Full hardware and configuration init every boot, which is the setting a bring-up wants |

### 1. Is VT-d enabled? Yes.

**IMG_4084, panel "VT for Direct I/O": the checkbox `Enable VT for Direct I/O` is ticked.** The
help text is Dell's own:

> This option specifies whether a Virtual Machine Monitor (VMM) can utilize the additional hardware
> capabilities provided by Intel® Virtualization Technology for Direct I/O.
>
> [NOTE: Trusted Execution requires VT for Direct I/O to be enabled.]

Its prerequisite is satisfied too: IMG_4083, `Enable Intel Virtualization Technology`, is ticked.

**What this settles.** `design/decisions/86-el0-nvme-driver.md` was decided on 2026-09-03 and its
research recorded that no board this project owns has an IOMMU in front of a real NVMe controller.
xenon has both, and now both are known to be *switched on*: VT-d enabled in firmware, and a
`Micron 2450 NVMe 256GB` on M.2 PCIe SSD-0 with SATA in AHCI rather than RAID mode, so the
controller is an ordinary PCIe function rather than something Intel RST has swallowed. The
confined-driver experiment §86 exists to enable has hardware it can actually run on.

**What it does not settle.** A ticked box means the firmware was asked to bring the DMAR up. It is
not the same as a DMAR table with the remapping units this kernel expects, and nothing here has
read one. The next thing worth doing at the bench is dumping the DMAR, which the boot tour's ACPI
table list already gets close enough to touch.

### 2. Is interrupt remapping on? Not shown, and the menu has no control for it.

**Say this plainly rather than infer it: none of the 70 pages mentions interrupt remapping, an
interrupt remap table, DMA protection, or an IOMMU by any name.** The whole of the Virtualization
Support menu is three pages, all photographed, and this is all of it:

- IMG_4083, `Virtualization`: `Enable Intel Virtualization Technology`, **ticked**
- IMG_4084, `VT for Direct I/O`: `Enable VT for Direct I/O`, **ticked**
- IMG_4085, `Trusted Execution`: `Trusted Execution`, **unticked**

So the question `notes/x86-uefi-boot.md` says only xenon can answer is **still open, and the
photographs are the wrong instrument for it.** A 7050 of this vintage exposes one VT-d switch and
no sub-controls; whether the remapping hardware reports the interrupt-remapping capability, and
whether firmware left it enabled, lives in the DMAR table's flags and in the unit's capability
register, not in a menu.

**That is a useful narrowing rather than a failure.** It converts an open question from "go to the
bench and photograph a screen" into "read the DMAR", which is code this kernel can run on its own
and a lane can write without a null modem. `notes/confinement-claims.md`'s fifth claim (a confined
component's MSI-X write is a memory write, so DMA remapping does not cover it, which is why VFIO
refuses userspace drivers without interrupt remapping) stays latent, and now has a defined next
step. A proposal is filed: `design/roadmap/378-read-the-dmar-on-xenon.md`.

### A number that disagrees with the boot tour, recorded rather than resolved

**Firmware's own System Information page says `Memory Installed = 16384 MB` and
`Memory Available = 16286 MB`** (IMG_4025), from two 8192 MB DDR4 DIMMs. `notes/x86-uefi-boot.md`'s
first-light account says the boot tour reported **17,119 MiB total** and calls the machine
"17 GB of RAM".

Both were read honestly off a screen and they do not agree. This note does not resolve it, because
resolving it means looking at what the kernel is actually summing over 148 memory-map regions and
that is code rather than a photograph. It is recorded here so that the next person to quote either
number knows the other exists.

**It does not disturb the `AlreadyMapped` hypothesis** in that note, which needs only that RAM
reaches the framebuffer aperture at `0xd0000000` (3.25 GiB). Sixteen gigabytes does that as
comfortably as seventeen.

### The hazard a headless xenon will hit

**`Warnings and Errors` is set to `Prompt on Warnings and Errors` and `Enable Keyboard Error
Detection` is ticked, so this machine stops at POST and waits for a keypress when it does not find
a keyboard.** Dell's own help text on that page names a headless configuration as the reason to
change it, and the machine's own event log (IMG_4093) carries **eight `Alert! Keyboard not found`
entries** across a year, so this is a thing it actually does rather than a thing it could do.

Nothing about this was hit during first light, because a keyboard and a monitor were both attached.
It matters for what `notes/bench-runbook.md` and `notes/serial-less-output.md` want next: a machine
that can be power-cycled and left to boot on its own. Two settings would have to change together
(`Continue on Warnings and Errors`, and keyboard error detection off), and both are an architect's
call because both change the machine's behaviour for everything else it is used for.

## The full transcription

Ordered as the menu tree is, which is also roughly the order of the `IMG_` numbers.

**The navigation tree**, legible in full only on IMG_4025 (every other frame crops or blanks it):
General, System Information, Boot Sequence, Advanced Boot Options, UEFI Boot Path Security,
Date/Time, System Configuration, Video, Security, Secure Boot,
Intel® Software Guard Extensions™, Performance, Power Management, POST Behavior, Manageability,
Virtualization Support, Wireless, Maintenance, System Logs, Advanced configurations.

### General

**IMG_4025, IMG_4026, System Information** (read-only)

| Field | Value |
|---|---|
| Bios version | 1.27.0 |
| Service Tag | 25XNBM2 |
| Asset Tag | (none) |
| Ownership Tag | (blank) |
| Manufacture Date | 12/22/2017 |
| Ownership Date | 03/21/2018 |
| Express Service Code | 4712411018 |
| | Signed Firmware Update is enabled |
| Memory Installed | 16384 MB |
| Memory Available | 16286 MB |
| Memory Speed | 2400 MHz |
| Memory Channel Mode | Dual |
| Memory Technology | DDR4 SDRAM |
| DIMM 1 Size / DIMM 2 Size | 8192 MB / 8192 MB |
| Slot1_M.2 | Mass Storage |
| Slot2_M.2 | Network |
| Processor Type | Intel(R) Core(TM) i5-7500T CPU @ 2.70GHz |
| Core Count | 4 |
| Processor ID | 906e9 |
| Current / Minimum / Maximum Clock Speed | 2.600 GHz / 0.800 GHz / 2.700 GHz |
| Processor L2 / L3 Cache | 1024 KB / 6144 KB |
| HT Capable | No |
| 64-Bit Technology | Yes (Intel EM64T) |
| SATA-0 | (none) |
| SATA-4 | (none) |
| M.2 PCIe SSD-0 | 256 GB 2319413C562A |
| LOM MAC Address | D8-9E-F3-74-B2-A2 |
| Video Controller | Intel HD Graphics |
| Audio Controller | RealTek ALC3234 |
| Wi-Fi Device | Intel Wireless |
| Bluetooth Device | Installed |

`HT Capable: No` is worth a second look: the i5-7500T has four cores and no hyperthreading, so the
four logical CPUs the boot tour enumerates are four physical ones.

**IMG_4027, Boot Sequence.** `Windows Boot Manager` present and **unticked**. Boot List Option:
Legacy ( ), **UEFI (•)**, and Legacy is greyed. Buttons: Add Boot Option, Delete Boot Option, View.

**IMG_4028, Advanced Boot Options.** `Enable Legacy Option ROMs` **unticked**;
`Enable Attempt Legacy Boot` **unticked** and greyed.

**IMG_4029, UEFI Boot Path Security.** **Always, Except Internal HDD (•)**; Always ( ); Never ( ).
Has no effect while no Admin password is set, and none is (IMG_4043).

**IMG_4030, Date/Time.** `09` / `04` / `26`, `07:07:45 PM`. The machine's clock at the moment of
photographing, not a setting.

### System Configuration

**IMG_4031, Integrated NIC.** `Enable UEFI Network Stack` **unticked**. Disabled ( );
**Enabled (•)**; Enabled w/PXE ( ). So the LAN is visible to an OS but there is no UEFI PXE path.

**IMG_4032, Serial Port.** Disabled ( ); **COM1 (•)**; COM2 ( ); COM3 ( ); COM4 ( ). Help text
verbatim:

> This option determines how the built-in serial port operates. It lets you avoid resource
> conflicts between devices by disabling or remapping the address of the port.
>
> COM1 = Port is configured at 3F8h with IRQ 4.
> COM2 = Port is configured at 2F8h with IRQ 3.
> COM3 = Port is configured at 3E8h with IRQ 4.
> COM4 = Port is configured at 2E8h with IRQ 3.
>
> Note: the operating system may allocate resources even though the setting is Disabled.

**IMG_4033, SATA Operation.** Disabled ( ); **AHCI (•)**; RAID On ( ).

**IMG_4034, Drives.** `SATA-0` **ticked**, `SATA-4` **ticked**, `M.2 PCIe SSD-0` **ticked**.
Details: SATA-0 Type `(none)`, Device ID `(none)`; SATA-4 Type `(none)`, Device ID `(none)`;
M.2 PCIe SSD-0 Type `256 GB SSD`, Device ID `Micron 2450 NVMe 256GB`.

**IMG_4035, SMART Reporting.** `Enable SMART Reporting` **ticked**.

**IMG_4036, USB Configuration.** `Enable USB Boot Support` **ticked**;
`Enable Front USB Ports` **ticked**; `Enable Rear USB Ports` **ticked**.

**IMG_4037, Front USB Configuration.** `Front Port 1 w/Power Share(Bottom)*` **ticked**;
`Front Port 2(Top)*` **ticked**.

**IMG_4038, Rear USB Configuration.** `Rear Port 1(Bottom)*`, `Rear Port 2(Lower Middle)*`,
`Rear Port 3(Upper Middle)*`, `Rear Port 4(Top)*` all **ticked**.

**IMG_4039, USB PowerShare.** `Enable USB PowerShare` **unticked**.

**IMG_4040, Audio.** `Enable Audio` **ticked**, with `Enable Microphone` **ticked** and
`Enable Internal Speaker` **ticked**.

**IMG_4041, Dust Filter Maintenance.** **Disabled (•)**; 15/30/60/90/120/150/180 days all ( ).

### Video

**IMG_4042, Primary Display.** **Auto (•)**; Intel HD Graphics ( ).

### Security

**IMG_4043, Admin Password.** Old password field greyed, reading **"Not Set"**. New and confirm
fields empty.

**IMG_4044, System Password.** Old password field greyed, reading **"Not Set"**. New and confirm
fields empty. So neither password is set, which is why IMG_4029 and IMG_4055 have no effect.

**IMG_4045, Strong Password.** `Enable Strong Password` **unticked**.

**IMG_4046, Password Configuration.** Admin Password Min `04`, Max `32`; System Password Min `04`,
Max `32`.

**IMG_4047, Password Bypass.** **Disabled (•)**; Reboot Bypass ( ).

**IMG_4048, Password Change.** `Allow Non-Admin Password Changes` **ticked**.

**IMG_4049, UEFI Capsule Firmware Updates.** `Enable UEFI Capsule Firmware Updates` **ticked**.

**IMG_4050, TPM 2.0 Security.** `TPM On` **ticked**; `PPI Bypass for Enable Commands` **ticked**;
`PPI Bypass for Disable Commands` **unticked**; `PPI Bypass for Clear Command` **unticked**;
`Clear` **unticked**; `Attestation Enable` **ticked**; `Key Storage Enable` **ticked**;
`SHA-256` **ticked**. Radio group below: Disabled ( ); **Enabled (•)**.

**IMG_4051, Computrace(R).** **Deactivate (•)**; Disable ( ); Activate ( ). The page states the
Absolute Anti-Theft solution is presently Deactivated, and that Activate and Disable are permanent.

**IMG_4052, Chassis Intrusion.** `Clear Intrusion Warning` **unticked**. Disabled ( );
Enabled ( ); **On-Silent (•)**. The page reports that an intrusion has been detected, which the
BIOS event log (IMG_4093) also shows repeatedly.

**IMG_4053, CPU XD Support.** `Enable CPU XD Support` **ticked**.

**IMG_4054, OROM Keyboard Access.** **Enabled (•)**; Disabled ( ); One Time Enable ( ).

**IMG_4055, Admin Setup Lockout.** `Enable Admin Setup Lockout` **unticked**.

**IMG_4056, Master Password Lockout.** `Enable Master Password Lockout` **unticked**.

**IMG_4057, SMM Security Mitigation.** `SMM Security Mitigation` **unticked**.

### Secure Boot

**IMG_4058, Secure Boot Enable.** **Disabled (•)**; Enabled ( ). Help text verbatim:

> This option enables or disables the Secure Boot feature. For Secure Boot to be enabled, the
> system needs to be in UEFI boot mode and the Enable Legacy Option ROMs option needs to be turned
> off.

**IMG_4059, Expert Key Management.** `Enable Custom Mode` **unticked**. Custom Mode Key
Management: **PK (•)**; KEK ( ); db ( ); dbx ( ). `Save to File` enabled; `Replace from File`,
`Append from File`, `Delete`, `Reset All Keys`, `Delete All Keys` all greyed out (Custom Mode is
off).

### Intel® Software Guard Extensions™

**IMG_4060, Intel® SGX™ Enable.** Disabled ( ); Enabled ( ); **Software Controlled (•)**.

**IMG_4061, Enclave Memory Size.** 32MB ( ); 64MB ( ); **128MB (•)**. No effect while SGX is
Software Controlled.

### Performance

**IMG_4062, Multi Core Support.** **All (•)**; 1 ( ); 2 ( ); 3 ( ).

**IMG_4063, Intel® SpeedStep™.** `Enable Intel® SpeedStep™` **ticked**.

**IMG_4064, C-States Control.** `C states` **ticked**.

**IMG_4065, Limit CPUID Value.** `Enable CPUID Limit` **unticked**.

**IMG_4066, Intel® TurboBoost™.** `Enable Intel® TurboBoost™` **ticked**.

SpeedStep, C-states and TurboBoost are all on, which is worth knowing before anybody quotes a
benchmark number off this machine: the clock is free to move between 0.800 and 2.700 GHz and the
cores are free to sleep.

### Power Management

**IMG_4067, AC Recovery.** Power Off ( ); Power On ( ); **Last Power State (•)**.

**IMG_4068, Auto On Time.** Time `12:00 AM`. **Disabled (•)**; Every Day ( ); Weekdays ( );
Select Days ( ). All seven day checkboxes unticked and greyed.

**IMG_4069, Deep Sleep Control.** Disabled ( ); Enabled in S5 only ( );
**Enabled in S4 and S5 (•)**. The help text warns that with this enabled, Remote Wakeup and Remote
Manageability are disabled while shut down or hibernating.

**IMG_4070, USB Wake Support.** `Enable USB Wake Support` **ticked**.

**IMG_4071, Wake on LAN/WLAN.** **Disabled (•)**; LAN Only ( ); LAN with PXE Boot ( );
WLAN Only ( ); LAN or WLAN ( ).

**IMG_4072, Block Sleep.** `Block Sleep (S3 State)` **unticked**.

**IMG_4073, Intel Ready Mode.** `Enable Intel Ready Mode` **unticked**.

### POST Behavior

**IMG_4074, Adapter Warnings.** `Enable Adapter Warnings` **unticked**.

**IMG_4075, Numlock LED.** `Enable Numlock LED` **ticked**.

**IMG_4076, Keyboard Errors.** `Enable Keyboard Error Detection` **ticked**.

**IMG_4077, Fastboot.** Minimal ( ); **Thorough (•)**; Auto ( ). Thorough performs complete
hardware and configuration initialisation during boot, which is the slow and safe end of this
setting and the right one for a bring-up.

**IMG_4078, Extend BIOS POST Time.** **0 seconds (•)**; 5 seconds ( ); 10 seconds ( ).

**IMG_4079, Full Screen Logo.** `Enable Full Screen Logo` **unticked**.

**IMG_4080, Warnings and Errors.** **Prompt on Warnings and Errors (•)**; Continue on Warnings ( );
Continue on Warnings and Errors ( ). Help text verbatim, because this one has consequences for a
headless bench (see "The hazard a headless xenon will hit", above):

> The Warnings and Errors options cause the boot process to only pause when warnings or errors are
> detected, rather than stop, prompt and wait for user input. This feature is valuable in
> situations where the system may be remotely managed and therefore has no locally connected
> keyboard or console available for a user to respond; for example, in a headless configuration.
>
> Users may select to have the Power On Self-Test (POST) process either a) stop, prompt and wait
> for user input when warnings or errors are detected, or b) continue when warnings are detected
> but pause on errors, or c) continue when either warnings or errors are detected during the POST
> process.
>
> Note that errors deemed critical to the operation of the system hardware will always halt the
> system.

### Manageability

**IMG_4081, USB Provision.** `Enable USB Provision` **unticked**.

**IMG_4082, MEBx Hotkey.** `Enable MEBx Hotkey` **ticked**.

### Virtualization Support

**IMG_4083, Virtualization.** `Enable Intel Virtualization Technology` **ticked**.

**IMG_4084, VT for Direct I/O.** `Enable VT for Direct I/O` **ticked**. Help text verbatim:

> This option specifies whether a Virtual Machine Monitor (VMM) can utilize the additional hardware
> capabilities provided by Intel® Virtualization Technology for Direct I/O.
>
> [NOTE: Trusted Execution requires VT for Direct I/O to be enabled.]

**IMG_4085, Trusted Execution.** `Trusted Execution` **unticked**.

### Wireless

**IMG_4086, Wireless Device Enable.** `WLAN/WiGig` **ticked**; `Bluetooth®` **ticked**.

### Maintenance

**IMG_4087, Service Tag.** `25XNBM2`.

**IMG_4088, Asset Tag.** Empty.

**IMG_4089, SERR Messages.** `Enable SERR Messages` **ticked**.

**IMG_4090, BIOS Downgrade.** `Allow BIOS Downgrade` **ticked**.

**IMG_4091, Data Wipe.** `Wipe on Next Boot` **unticked**. Leave it that way: the page's own
warning is that the operation is not recoverable and cannot be terminated once started, across
internal SATA HDD/SSD, M.2 SATA SSD, M.2 PCIe SSD and eMMC.

**IMG_4092, BIOS Recovery.** `BIOS Recovery from Hard Drive` **ticked**;
`BIOS Auto-Recovery` **unticked**. Both are the firmware's stated defaults.

### System Logs

**IMG_4093, BIOS Events.** Twenty entries, oldest at the bottom of the visible list. Verbatim:

```
09/04/2026  22:00:09   Alert! Cover was previously removed.
09/04/2026  22:04:58   Alert! Cover was previously removed.
08/09/2025  19:45:43   Alert! Keyboard not found.
08/09/2025  20:19:41   Alert! Keyboard not found.
08/09/2025  20:28:37   Alert! Keyboard not found.
08/19/2025  13:47:59   Alert! Keyboard not found.
06/20/2026  18:00:39   Alert! Keyboard not found.
07/30/2026  00:42:02   Alert! Keyboard not found.
08/15/2026  10:07:36   Alert! Cover was previously removed.
08/15/2026  10:08:18   Alert! Cover was previously removed.
08/15/2026  10:08:42   Alert! Cover was previously removed.
08/15/2026  10:09:11   Alert! Cover was previously removed.
08/15/2026  10:09:30   Alert! Cover was previously removed.
08/15/2026  10:10:01   Alert! Cover was previously removed.
08/15/2026  10:15:38   Alert! Cover was previously removed.
08/15/2026  10:18:22   Alert! Cover was previously removed.
08/15/2026  10:19:48   Alert! Cover was previously removed.
08/15/2026  10:21:56   Alert! Cover was previously removed.
08/28/2026  06:00:05   Alert! Cover was previously removed.
08/28/2026  06:00:06   Alert! Keyboard not found.
```

The list is not in date order on the screen and is reproduced as it appeared. The two 09/04/2026
entries at the top are the first-light session itself. `Alert! Cover was previously removed` is
IMG_4052's `On-Silent` chassis intrusion setting doing what it says, and it will keep appearing
until somebody ticks `Clear Intrusion Warning`.

### Advanced configurations

**IMG_4094, ASPM.** Auto ( ); **Disabled (•)**; L1 Only ( ).

## What is not shown

Seventy photographs of a menu tree are not necessarily all of it, and this section is the honest
list rather than a disclaimer.

- **No interrupt-remapping control exists anywhere in these pages**, and the whole Virtualization
  Support submenu is photographed, so this is an absence in the firmware rather than a gap in the
  photography. See question 2 above.
- **The navigation tree is legible on exactly one frame** (IMG_4025). Every other photograph either
  crops the left pane at the frame edge or shows it blank. So the menu structure above is
  reconstructed from that single frame plus the order the pages were photographed in, and a
  submenu that was never opened would leave no trace.
- **`POST Behavior` and `Manageability` pages are numbered in this note but their contents are
  carried in the entries themselves**, not in the section headers; if a page of either menu was
  never opened it does not appear here at all.
- **No page for the F12 one-time boot menu is in this set.** That is
  `art/bench/xenon-2026-09-05-boot-menu.jpg`, recorded in `notes/x86-uefi-boot.md`, and it is where
  the `UEFI: SanDisk Ultra 1.26` and `UEFI: Micron 2450 NVMe 256GB` entries come from.
- **No DMAR, no ACPI table contents, no PCI enumeration.** None of that is in a firmware menu. The
  boot tour prints some of it and `notes/x86-uefi-boot.md` records what was seen.
- **Help text is transcribed verbatim only where it carries a fact** (the COM port addresses, the
  SATA modes, the Secure Boot precondition, the VT-d note). Everywhere else it is summarised or
  omitted, because Dell's help text is the same on every 7050 and the *setting* is what is specific
  to this machine.

## BUGS

- **This is a record of 2026-09-04 and nothing keeps it true.** No gate compares it to the machine,
  and no gate could: the machine is not on a network any lane can reach. A setting changed at the
  bench and not written here makes this note confidently wrong, which is worse than absent.
- **Checkbox state was read from photographs of a screen, at an angle, in a dim room.** A ticked and
  an unticked box differ by a few pixels of glyph. The settings in the "What nife depends on" table
  were each re-read from the original frame by a second pass; the long transcription below it was
  not, and a single-character error is likelier there than in the table.
- **The two memory figures in this tree disagree** and this note does not resolve which is right;
  see "A number that disagrees with the boot tour" above.
- **A ticked `Enable VT for Direct I/O` is a firmware intention, not an observed IOMMU.** Nothing in
  this tree has read xenon's DMAR. The proposal that would is
  `design/roadmap/378-read-the-dmar-on-xenon.md`.
