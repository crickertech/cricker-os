# xenon, 2026-10-04 (UTC): fatal risk 6's first silicon runs

Transcribed from calef's photographs of xenon's screen, relayed by the maintainer. There is no
serial on this bench, so these logs are the record; the photographs are not committed. Each log
holds only the lines that were readable and relayed, in screen order. Read with
`notes/risk-6-bench-evening.md`.

| Log | Image | What it showed |
|---|---|---|
| `boot-a-main.log` | main at a08efc8dc, `cargo xtask disk-throughput --stage-only` | the screen tore into a grid as `vt-d drhd ... up` printed; nothing after it readable |
| `boot-b-pre594.log` | 7ae6d4e15 (before milestone 594 (every VT-d unit translates its own devices)), sha256 `65dc956e..` | preflight 1 PASS, then bring-up `CompletionTimeout` |
| `boot-c-diag.log` | `lane/xenon-nvme-diag-pre594` at 3dfd2e813, sha256 `d1bd5082..` | the diagnosis: VT-d fault reason 0x01 on the NVMe's admin queue, `ECAP.C` = 0 |
| `boot-d-wbinvd.log` | `lane/xenon-nvme-diag-pre594` at `d3dbe8cf253d33f648808393ce89063983583c12`, sha256 `e583b593..` | the write-back worked: the fault moved to reason 0x0b, a reserved field in the context entry (domain 0x100 on an 8-bit unit) |
| `boot-e-main-clflush-1.log` | main plus the table write-back, sha256 `ac4604ed..` | bench boot 1 of 3: screen held, both preflights PASS, `CONFINED-AT-RATE` |
| `boot-f-main-clflush-2.log` | the same image | bench boot 2 of 3: screen held, both preflights PASS, `CONFINED-AT-RATE` |
| `boot-g-main-clflush-3.log` | the same image | bench boot 3 of 3: screen held, both preflights PASS, `CONFINED-AT-RATE` |
