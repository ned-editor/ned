# ned — Not Exactly an IDE

A pure, ultra-fast modal terminal text editor written in Rust. No language servers,
no background daemons, no bloat. Just you, your keyboard, and your files.

---

## What it is

Ned is a Helix-inspired terminal editor that strips everything back to what actually
matters for editing text. It is **selection-first** — you select a thing, then act on
it — which makes complex edits feel natural once it clicks. The whole surface area is
the keyboard. The mouse is optional.

It starts in **~2 ms** and peaks at **~12 MB RSS** on a cold release run. That is not
a benchmark cherry-pick; it is what you get every time because there is simply nothing
running in the background.

---

## What you get

- **Modal editing** — Normal, Insert, Select, and Command modes with a coherent motion
  language borrowed from the Helix lineage.
- **Syntax highlighting** — Powered by JSON tree-sitter grammars. Ships with Rust,
  Python, TOML, JSON, and Markdown out of the box.
- **Themes** — Comes with `ned-dark` and `ned-light`. Drop any TOML theme file into
  your runtime directory and it is picked up automatically.
- **Git gutter** — Added (`+`), modified (`~`), and removed (`-`) lines shown inline.
  Navigate hunks with `]g` / `[g`.
- **Regex search** — Full regex search and replace with live highlighting.
- **Language-aware indentation and comments** — Newlines auto-indent to context;
  `gc` and `gC` toggle line and block comments per language.
- **Config and runtime discovery** — Looks in `.ned/` at the repo root, then your user
  config directory, then falls back to built-in defaults. It always starts.

---

## Optional external tools

Ned works without any of these. They unlock specific features when present on `PATH`:

| Tool | What it unlocks |
|---|---|
| `git` | Diff gutter and branch name in the status line |
| `curl` | `--fetch-themes` and `--fetch-grammars` CLI flags |
| `wl-copy` / `xclip` / `pbcopy` / `termux-clipboard-set` | System clipboard integration |

---

## CLI flags

```
ned [file]            open a file (supports file:line and +line syntax)
ned --health          print runtime paths and tool availability
ned --fetch-themes    download community themes into your runtime directory
ned --fetch-grammars  download grammar files for additional languages
ned --theme <name>    start with a specific theme
ned --config <path>   use a custom config file
ned --no-config       skip config entirely and run with defaults
ned --version         print version and exit
```

---

## Platform support

Developed and tested on **Linux**. Expected to work on **macOS** and **Termux**.
Windows support is best-effort — the core editor works in any VT-compatible terminal,
though some platform paths and clipboard providers may need adjusting.

---

## On the Helix codebase

The `fork/` directory in this repository contains a verbatim copy of the
[Helix](https://github.com/helix-editor/helix) source tree. It is kept as a local
reference — not vendored into the build — and is used for studying code logic,
understanding engineering decisions, and porting specific algorithms (tree-sitter
integration, Myers diff, regex engine wiring, motion grammar). Ned is an independent
Rust project; the fork carries no runtime dependency and is never shipped.

---

## Stability

Ned ships pre-compiled release binaries. A binary is published only after the full
build passes, but the project is **fully experimental** right now. Things can and do
break at certain points. The editor is mostly usable for day-to-day editing, and
stability is improving steadily — but do not bet a production workflow on it yet.
Treat every release as a preview.

---

> Built because fast, focused tools deserve to exist. No plugin ecosystem, no roadmap
> to become an IDE. Just an editor.
