---
status: NOT-STARTED
raised: 2026-09-29
promoted_from: usb-keyboards-behind-the-console
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 618. USB keyboards behind the console

The console's input today is the UART (PL011 on `virt`, the x86 serial port), the raw receive
driver that milestone 19f (run a real workload) built. The display ladder has `display_terminal`,
but input does not follow it off the serial line, and every physical machine a stranger would boot
hands us a USB keyboard.

This milestone adds xHCI plus HID input for the console path only: keyboards and tablets.
Tablets and keyboards are the devices the compositor's input path needs. Explicitly out until their
consumers exist: mass storage, hubs beyond the root, and anything that is not input. A device row
follows its driver, and this is the driver; the boot matrix's USB present/absent row on `q35`
unlocks when this lands.

Name provisional; the driver lands in the kernel's driver set with what it needs passed in, per
the codebase rules.

## Index row

The console's input is the serial UART today. Every machine a stranger would boot hands us a USB
keyboard instead. This milestone brings xHCI and HID input to the console path, input devices only,
and leaves mass storage out. It is the driver behind the boot matrix's USB device row.
