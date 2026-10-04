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
