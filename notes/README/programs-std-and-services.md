# Notes index: Programs, std and services

What runs at EL0: the std port, the shell, components, and the services they call.

Part of [the notes index](../README.md), which says how to add a line.

- [Rust `std` on the native ABI](../std.md): std's platform layer implemented on the capability ABI.
- [Somebody else's crate on nife](../crates-io-on-nife.md): fifty crates.io crates built against nife's `std`.
- [`ripgrep` on nife](../ripgrep-on-nife.md): unmodified ripgrep builds and runs, and what stops it.
- [What one shim costs](../foreign-program-arguments.md): priced per program and as a library.
- [A TLS crypto provider on nife](../cryptography-provider.md): building a `rustls` crypto provider for all three targets.
- [The `thread::spawn` fork](../thread-spawn-fork.md): what a std thread would cost, and why declined.
- [Running a foreign language: the C seam](../c-seam.md): a confined, restartable C component under a Rust shell.
- [The program manifest](../program-manifest.md): a program's declared endowment, checked at spawn.
- [A shell at EL0](../shell.md): an interactive shell, console input and spawned workers.
- [The line discipline as a userspace component](../line-discipline.md): the tty line editor as a userspace process.
- [The terminal contract](../terminal-contract.md): the IPC protocol a terminal presents to programs.
- [The sink protocol](../sink-protocol.md): one register-only protocol for writing bytes anywhere.
- [Pipes and redirection](../pipes.md): `>`, `<` and `|` as one capability substitution.
- [The tail-stage output fork](../tail-output-narrowing.md): where a tail stage's output goes, decided.
- [`swish` the language](../swish-language.md): quoting, sequencing, and refusal as its own exit status.
- [The command line as a grant expression](../grant-expression.md): naming a resource at the prompt grants it.
- [The glob matcher](../glob.md): a pure byte glob matcher with a bounded cost.
- [Globbing, and the expansion you see is the grant](../glob-grant.md).
- [Navigating with no global namespace](../shell-navigation.md): `cd`, `pwd`, `ls`, `mkdir` and `rm` as capability builtins.
- [The inert-configuration page](../env-config.md): validated read-only `TZ`, `LANG` and `TERM` for programs.
- [The documentation crate](../documentation.md): streaming markdown renderer, manual viewer and search index.
- [The component manifest](../component-manifest.md): what a supervisor must route before a component serves.
- [Live component replacement](../live-replacement.md): swapping a running component under a live client.
- [The hung component](../hung-component.md): a component that stops answering without dying.
- [Dependency-aware orchestration](../dependency-orchestration.md): which components to warn before swapping a dependency.
- [The process view](../process-view.md): `ps`, `pgrep` and `top` over a supervision subtree.
- [Scheduled execution](../scheduled-execution.md): a cron whose every entry is a grant.
- [Durable delegation](../durable-delegation.md): a login session that outlives its client.
- [Wall-clock time](../clock.md): wall clock as counter plus offset, three authorities.
- [`date`](../date.md): prints the wall clock and cannot set it.
- [`time`](../time-command.md): times a command on the shell's clock.
- [The calendar crate](../calendar.md): Unix seconds to civil dates and back, and formats.
- [Entropy](../entropy.md): where randomness comes from, and who may reach it.
- [Credentials](../credentials.md): checking a secret the service can never read back.
- [NTLM](../ntlm.md): the NTLMv2 key half of the secrets store, removed.
- [Login](../login.md): authentication that returns capabilities instead of changing identity.
- [The network stack as a confined component](../net.md): the confined NIC driver, smoltcp server and socket contract.
- [NTP: the wire format, and the client that carries it](../ntp.md): the NTPv4 codec and a one-shot time client.
- [SMB: the network file service a Mac mounted, and why it is no longer here](../smb.md).
- [mDNS/DNS-SD: the Time Machine advertisement, and why it is no longer here](../mdns.md).
