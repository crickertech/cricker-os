# Other systems and compatibility layers for Wi-Fi

Appendix to [788. Wi-Fi on a PC that has no Ethernet](../788-wifi-on-a-pc-that-has-no-ethernet.md).

## Rust operating systems

Redox has no wireless driver (`redox-os/drivers/net` holds `alxd`, `e1000d`, `ixgbed`, `rtl8139d`
and `virtio-netd`). No Wi-Fi was found for Theseus, Tock or Hubris; that is not found rather than
proven absent. Embassy's `cyw43` runs the WPA handshake inside the chip's firmware, which the 8265
does not offer. `supplicant-rs` (Apache-2.0) does the 4-way handshake and SAE, but over Linux
nl80211 and tokio.

## Driver-compatibility layers, and why each lands in a package

| layer | what it runs | license of the result | verdict |
|---|---|---|---|
| Genode `dde_linux`, `pc_wifi` | Linux 6.18.19's iwlwifi, mac80211 and (implied by its config format) `wpa_supplicant` | GPLv2 through mac80211; Genode itself is AGPLv3 | package only under §135, so useless at install time |
| TU Dresden DDE/DDEKit | Linux 2.6 drivers | GPLv2 for the Linux part (from a search snippet, not read) | stale, and GPL |
| NetBSD rump kernels | `libnet80211` and only `iwn` among wireless drivers; no rump `iwm` | BSD (not read) | NetBSD's `iwm` does 802.11a/b/g only |
| FreeBSD LinuxKPI `iwlwifi` | Linux iwlwifi under a shim over net80211 | the BSD half of iwlwifi | lists the 8265, but 802.11n/ac only on 22000 and later |

A compatibility layer is the fastest way to a working Wi-Fi package and the one route that cannot
serve the base. Genode tracks recent Linux and would bring every Intel generation at once, a real
argument for a later GPL package but not for install time.
