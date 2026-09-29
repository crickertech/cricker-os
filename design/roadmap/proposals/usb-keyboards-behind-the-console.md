---
status: PROPOSED
raised: 2026-09-29
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# USB keyboards behind the console

The console's input today is the UART (PL011 on `virt`, the x86 serial port; the raw receive
driver, milestone 19f.4). The display ladder has `display_terminal`, but input does not follow it
off the serial line, and every physical machine a stranger would boot hands us a USB keyboard.

This mints xHCI plus HID input for the console path only: keyboards and tablets, the devices the
compositor's input path needs. Explicitly out until their consumers exist: mass storage, hubs
beyond the root, and anything that is not input. A device row follows its driver, and this is the
driver; the boot matrix's USB present/absent row on `q35` unlocks when this lands.

Name provisional; the driver lands in the kernel's driver set with what it needs passed in, per
the codebase rules.
