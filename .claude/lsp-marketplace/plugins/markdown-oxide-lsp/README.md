# markdown-oxide-lsp

Registers [markdown-oxide](https://github.com/Feel-ix-343/markdown-oxide) as the Claude Code LSP server for `.md`, giving the LSP tool (and your editor):

- **goToDefinition** on a `[[wikilink]]` → jump to the target note/heading
- **findReferences** on a note/heading → **backlinks** (every inbound link)
- **diagnostics** → unresolved/broken `[[links]]`
- **documentSymbol / workspaceSymbol** → headings and notes

## Requires the `markdown-oxide` binary on PATH
Not published to crates.io — build from git or use a system package:

```bash
cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
# or: pacman -S markdown-oxide · brew install markdown-oxide · apk add markdown-oxide · nix pkgs.markdown-oxide
```

Vault root is detected via `.git`, `.obsidian`, or `.moxide.toml`; this repo's `docs/.obsidian/` scopes it to the docs vault.

Registered for the repo via `.claude/settings.json` (`extraKnownMarketplaces` + `enabledPlugins`); enabled automatically once a collaborator trusts the workspace.
