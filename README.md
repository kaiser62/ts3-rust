# ts3-rust

Native Rust **TeamSpeak 3 Client** (`ts3-gui` 1:1 Desktop Client + `ts3-cli` Interactive CLI & Protocol Engine), built from reverse-engineered protocol and UI specifications of TeamSpeak 3 Server (`3.13.x`) and the official Windows TeamSpeak 3 Client.

## Workspace Crates

| Crate | Binary / Lib | Description |
| :--- | :--- | :--- |
| [`crates/proto`](./crates/proto) | `ts3-proto` (lib) | Wire protocol framing (13B C2S / 11B S2C), `TS3INIT1` 4-step handshake + RSA puzzle solver, `initivexpand2` (Ed25519 license chain root key derivation + X25519/Ed25519 ECDH), AES-128-EAX (8-byte MAC), LibTomCrypt ASN.1 DER P-256 `omega` identity + Hashcash PoW, QuickLZ Level 1 compressor/decompressor, command builder/parser. |
| [`crates/client`](./crates/client) | `ts3-client` (lib) | Async UDP client engine (`Ts3Client`, `Ts3ClientHandle`), 9-stream sequence tracker, `GenerationWindow`, RFC 6298 RTO retransmission queue, fragment reassembler, keepalive ping/pong, channel/client/server state sync. |
| [`crates/cli`](./crates/cli) | `ts3-cli` (bin) | Command-line client with interactive REPL (`connect`), automated live server verification (`smoke-test`), and P-256 identity generator (`identity`). |
| [`crates/gui`](./crates/gui) | `ts3-gui` (bin) | 1:1 Windows TeamSpeak 3 Desktop GUI in `eframe`/`egui` (`Ui_MainWindow` 3-pane splitter layout, Light & Dark QSS themes, `ServerTreeView` with spacer channels & badges, `.tpl` `InfoFrame`, BBCode `MainWindowChatWidget`, runtime SVG/flag/watermark/sound pack loader, and modal dialogs). |

## Reverse Engineering Documentation (`docs/re/`)

- **Server & Wire Protocol**:
  - [`docs/re/00-protocol-overview.md`](./docs/re/00-protocol-overview.md) — UDP protocol architecture, SQLite schema, 5-tier permission hierarchy
  - [`docs/re/01-binary-strings.md`](./docs/re/01-binary-strings.md) — ELF `.rodata` strings, NIST P-256 curve params, `LicenseRootKey`, dummy AES key/nonce, 238 error codes
  - [`docs/re/02-commands.md`](./docs/re/02-commands.md) — 145+ C2S commands, 72 S2C `notify*` events, escaping table, pipe-list header inheritance
  - [`docs/re/03-crypto-handshake.md`](./docs/re/03-crypto-handshake.md) — `TS3INIT1`, RSA puzzle, `initivexpand2` Ed25519 license chain, AES-128-EAX
  - [`docs/re/04-transport.md`](./docs/re/04-transport.md) — Packet headers, 9 sequence counters, `GenerationWindow`, QuickLZ Level 1
  - [`docs/re/05-voice.md`](./docs/re/05-voice.md) — Voice/Whisper payloads, Opus Voice (`4`) & Opus Music (`5`) framing
- **Desktop Client GUI**:
  - [`docs/re/gui/01-exe-qt-internals.md`](./docs/re/gui/01-exe-qt-internals.md) — 71 `Ui_*` Qt5 form classes, 280+ RTTI classes, shortcuts
  - [`docs/re/gui/02-mainwindow-layout.md`](./docs/re/gui/02-mainwindow-layout.md) — 3-pane splitter, 7 top-level menus, toolbar, spacer rules (`[spacer]`, `[cspacer]`, `[rspacer]`, `[*spacer]`)
  - [`docs/re/gui/03-qss-themes.md`](./docs/re/gui/03-qss-themes.md) — Light (`default.qss`) & Dark (`default_darkmode.qss`) palettes and chat colors
  - [`docs/re/gui/04-info-templates.md`](./docs/re/gui/04-info-templates.md) — `serverinfo.tpl`, `channelinfo.tpl`, `clientinfo.tpl`
  - [`docs/re/gui/05-gfx-icons.md`](./docs/re/gui/05-gfx-icons.md) — 209 SVGs in `default_colored_2014.zip` + 247 flags in `countries.zip`
  - [`docs/re/gui/06-dialogs-menus.md`](./docs/re/gui/06-dialogs-menus.md) — Modal dialogs & context menus
  - [`docs/re/gui/07-sound-packs.md`](./docs/re/gui/07-sound-packs.md) — 101 sound notification keys in `sound/default/settings.ini`
  - [`docs/re/gui/08-settings-chat-bbcode.md`](./docs/re/gui/08-settings-chat-bbcode.md) — `settings.db` schema, BBCode & `client://` / `channelid://` URI formats

## Quick Start

```bash
# Run all 102 unit & integration tests
cargo test --workspace

# Launch the 1:1 Desktop GUI
cargo run --release -p ts3-gui -- --server 127.0.0.1:9987 --nick "RustGUI"

# Run the CLI interactive REPL
cargo run --release -p ts3-cli -- connect --server 127.0.0.1:9987 --nick "RustClient"
```
