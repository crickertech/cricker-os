---
status: DECIDED
raised: 2026-09-25
decided: 2026-09-25
ratified_by: calef
---

# 215. The second RISC-V machine is a rented Scaleway Elastic Metal RV1

calef, 2026-09-25 (UTC). *(Section number provisional until the merge queue lands it.)* This closes
the provider half of [§203 (capacity is rented rather than bought)](203-capacity-is-rented-not-bought.md)'s
"What is not decided" for one purpose. It does not settle the spend split, and it does not choose a
provider for aarch64 or x86_64.

## The ruling

Rent one Scaleway Elastic Metal RV1 (`EM-RV1-C4M16S128-A`, a T-Head TH1520 with four C910 cores).
It is the machine for fatal risk 9's implementation-grain experiment: a second machine of an
architecture nife already boots. The port is milestone 89 (Scaleway EM-RV1: a second RISC-V
implementation, rented).

The machine was not rented on the day of the ruling. Milestone 89 orders its host and QEMU steps
first, so that paid hours begin at a step that needs the machine. It also recommends hourly
billing and deleting the server between sessions. Those are the maintainer's recommendations, not
part of the ruling.

## Why this machine

A second copy of radon would answer nothing. Two JH7110s share every assumption this tree could
have baked into one vendor's silicon. The experiment needs a different implementation of riscv64,
and the TH1520 differs from the JH7110 in the places a hidden assumption would live. Its peripherals
sit near the top of a 40-bit physical space, its DRAM starts at zero, its cores lack Sstc and
Svpbmt, and its page tables carry T-Head's own memory-attribute bits. Milestone 89 lists each one
against the code it touches.

## What else was considered

- Scaleway is the only rentable riscv64 bare metal that meets
  [`notes/rented-metal.md`](../../notes/rented-metal.md)'s four hard requirements: own kernel, serial
  console, power API, real cores. That survey was read 2026-09-23. AWS, Azure, Hetzner and OVH
  offer no riscv64 metal at all.
- RISE's free RISC-V GitHub runners run on the same RV1 hardware, but as Kubernetes pods. A pod
  cannot boot a kernel, so this fails requirement 1. It stays useful for a native build leg.
- Cloud-V's landing page, read 2026-09-25, offers SSH shells on shared boards and a LAVA farm for
  firmware tests. It names no boards, publishes no price, and documents no self-service path to
  boot a custom kernel with a console and a power switch. Not qualifying as read.
- Buying a second board, such as a Lichee Pi 4A with the same TH1520. §203 refused buying, and a
  bought board puts a person back at the bench. Renting the RV1 comes with the power API and a
  console on an SSH gateway.

## The offer, as read 2026-09-25

Sources: the [product page](https://www.scaleway.com/en/elastic-metal-rv1/), the
[Labs page](https://labs.scaleway.com/en/em-rv1/), the public product-catalog API
(`api.scaleway.com/product-catalog/v2alpha1/public-catalog/products`), and Scaleway's
[RV1 guidelines](https://www.scaleway.com/en/docs/elastic-metal/reference-content/elastic-metal-rv1-guidelines/),
read from `github.com/scaleway/docs-content` at commit `14fcd2023`.

| | |
|---|---|
| Price | €0.042 an hour or €15.99 a month, excluding VAT. Monthly carries a one-month commitment fee |
| Zone | `fr-par-2` only. The catalog lists it as `general_availability` in range `Labs` |
| Stock | not visible without an API key; `scw baremetal offer list zone=fr-par-2` shows it |
| SLA | 0%, a Labs service |
| Quota | 5 servers with a validated payment method; identity validation raises it to 10 |
| Own kernel | a U-Boot FIT image, `boot.itb`, on a FAT32 partition of the GPT eMMC, carrying `kernel`, `fdt`, `opensbi` and `env` |
| Firmware | U-Boot fixed in eMMC and not customer-modifiable; OpenSBI is the customer's, inside the FIT |
| Console | *"a serial console accessible via SSH is activatable on your account"*; how to activate it is not documented |
| Power | the Elastic Metal API's reboot (normal or rescue), start and stop |

Hourly beats monthly below about 380 hours a month. Milestone 89's bring-up is priced in hours, not
months, so hourly is the plan.

## What is not decided

- The U-Boot version and the vendor boot chain before it. Scaleway does not publish them; the first
  rented hour reads them off the console.
- How the serial console is activated. It may be a support request, which is calef's to file.
- Whether this machine later carries a standing gate. A 0% SLA means any gate on it must degrade to
  a loud skip, as [`design/roadmap/556-a-second-risc-v-implementation-for-rent.md`](../roadmap/556-a-second-risc-v-implementation-for-rent.md)
  said when it was filed.
