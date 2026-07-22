# Contributing — dev environment

## Docs tooling (Obsidian vault in `docs/`)

`docs/` is an Obsidian vault. Two tools give agents and editors first-class access to it. Both are wired up by committed repo config — you only need to install the one binary below.

### 1. Docs search MCP — automatic
`.mcp.json` registers `obsidian-docs` ([mcpvault](https://github.com/bitbonsai/mcpvault), runs via `npx`, no install). In Claude Code it loads after you trust the workspace — approve it when prompted. Gives content search + frontmatter/tag queries over the vault.

### 2. Markdown LSP — install the binary, then automatic in Claude Code
[markdown-oxide](https://github.com/Feel-ix-343/markdown-oxide) provides wikilink go-to-definition, backlinks (find-references), and broken-link diagnostics.

**Install the binary** (not on crates.io — build from git or use a system package):
```bash
cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
# or: pacman -S markdown-oxide · brew install markdown-oxide · apk add markdown-oxide · nix pkgs.markdown-oxide
```
Make sure it's on `PATH` (e.g. `~/.cargo/bin`).

**Claude Code**: the repo's `.claude/settings.json` registers the local plugin marketplace (`.claude/lsp-marketplace/`) and enables `markdown-oxide-lsp` — it activates automatically once you trust the workspace. No `/plugin` commands needed.
_Fallback if it doesn't appear: `/plugin marketplace add ./.claude/lsp-marketplace` then `/plugin install markdown-oxide-lsp@exergon-lsp`, then `/reload-plugins`._

**Other editors** (Neovim ≥0.11 / VS Code / Zed / Helix): configure `markdown-oxide` as the LSP for markdown in your editor — it's the same server, so you and the agent share one link graph. See the [markdown-oxide docs](https://oxide.md/).

### Keeping the vault healthy
After adding/removing/re-describing a doc, regenerate the frontmatter-driven index catalogs:
```bash
python3 scripts/gen_docs_index.py
```
