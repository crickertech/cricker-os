# A USB keyboard: the xHCI driver and what it is allowed to touch

Milestone 242 (USB host and HID, because on commodity hardware the keyboard is not a UART). How a
key pressed on a USB keyboard reaches the shell, what each piece holds, and how to prove it works.

## The path a key takes

```text
USB keyboard ──► xHCI controller ──DMA, confined by the IOMMU──► usb_keyboard_driver (EL0)
                                                                   │ OPERATION_BYTES, CALL
                                                                   ▼
                                                              line_editor ──► swish
```

The driver diffs each eight-byte boot report against the last, turns the difference into Linux
evdev key codes (`crates/usb`'s `boot_keyboard`), and feeds those through `video_terminal::keymap`,
the same layout table the virtio keyboard uses. The bytes go to the line discipline's endpoint in
the framing `input` (the UART) already uses, so the shell cannot tell the two apart and both work at
once.

## Who does what

| Piece | Where | Holds |
|---|---|---|
| Find the controller, place its BAR, route its interrupt (MSI-X, MSI, or INTx) | `kernel/src/pci.rs`, `find_xhci_device` | config space |
| Take it from the PC firmware (USB Legacy Support), draw the register window, confine its DMA | `kernel/src/extensible_host_controller_interface.rs` | EL1 |
| What the driver is handed and refused | `kernel/src/user/usb_keyboard_service.rs` | the `Spawn` literal |
| Reset, rings, enumeration, the keyboard | `components/src/usb_keyboard_driver.rs` | EL0 |
| Descriptors, setup packets, boot reports | `crates/usb` | host-tested, Kani |
| Register window, handoff, TRBs, rings, contexts, `PORTSC` | `crates/extensible_host_controller_interface` | host-tested, Kani |
| Where the line discipline's endpoint comes from | `crates/system_initializer`, step 3b | the progenitor |

The kernel starts the driver before the progenitor and waits for its one bring-up report, which it
prints as the boot's `usb       :` line. The progenitor is granted the driver's attach endpoint at
slot 27 and sends `WRITE` on the terminal endpoint through it once the line discipline exists.

## The confinement, in one sentence

The driver holds every register page of its controller except the page its interrupt message is
programmed through, and its controller can reach nothing but its own DMA region. The first half is
`register_window` (a Kani harness proves no withheld page is ever mapped). The second is the IOMMU.
The kernel refuses to start the driver at all on a machine whose IOMMU does not confine the
controller, because a process holding an xHCI register file names every physical address the
controller reads.

## Proving it

`script/swish-check` boots each architecture a fourth time with `qemu-xhci` and `usb-kbd` attached
and nothing else that could type, presses `echo hello` through the QEMU monitor, and requires
`hello` back. The keyboard is full speed (`usb_version=1`), as real keyboards are. On x86_64 the
controller has MSI and no MSI-X (`msix=off,msi=on`), as xenon's Intel PCH controller does.

To boot one by hand:

```sh
NIFE_USB_KEYBOARD=1 NIFE_SCREEN_MON=/tmp/mon.sock helpers/qemu-bounded.sh 120 \
    helpers/qemu-runner-aarch64.sh target/aarch64-unknown-none-softfloat/debug/kernel
# in another terminal, once the prompt is up:
echo 'sendkey e' | nc -U /tmp/mon.sock
```

`NIFE_USB_KEYBOARD_OPTS` and `NIFE_USB_CONTROLLER_OPTS` are appended to the two `-device` options.

## Per architecture

| | aarch64 | riscv64 | x86_64 |
|---|---|---|---|
| Under QEMU | INTx, SMMUv3; green | INTx, riscv-iommu; green | MSI (and MSI-X), VT-d; green |
| Real silicon | argon: not tried | radon: refused, no IOMMU | xenon: calef's bench step |

## BUGS

- Root ports only: a keyboard behind a hub is not found, and neither is a keyboard with a hub
  built into it. `crates/extensible_host_controller_interface`'s BUGS.
- One controller, one keyboard. The first xHCI on the bus, the first boot keyboard on it.
- No key repeat, no Caps Lock, no LEDs. `crates/usb`'s BUGS.
- No IOMMU, no USB keyboard. radon has none; a machine whose VT-d tables give the controller to
  a unit this kernel did not bring up gets the refusal line. This is the design, and it is a cost.
- A device refused after boot is refused silently: the report endpoint carries one message.
- Never run on silicon. Every number here is QEMU's: timeouts are the specification's limits,
  not measurements, and QEMU cannot model a low-speed device.
- **Keystrokes stall mid-line about one boot in twenty on riscv64** (found 2026-10-05 UTC, chasing
  a red `swish-check` on #1630). The USB leg types `echo hello` and the echo stops partway (after
  `e`, `echo `, `echo hell`), and later USB keys change nothing for 30 s. It is not the driver
  losing keys: a line typed on the UART afterwards releases every held USB byte, interleaved with
  the UART's. So the bytes reach the terminal endpoint, and `line_editor` is not woken for the
  USB driver's `call` until a second sender (`input`) arrives. That points at a lost wakeup on a
  two-sender rendezvous across harts, not at xHCI. Measured on `main` at 32dd00175: 3 failures in
  60 boots of that leg alone (riscv64, TCG, 4 harts); aarch64 and x86_64 not yet counted. Owed:
  a lane to find the race, which a real keyboard on xenon would also hit.
