# ZisK Assembly — VS Code syntax highlighting

Syntax highlighting for ZisK assembly (`.zisk`) source files (see
[`../ziskasm.md`](../ziskasm.md) for the language).

It highlights:

- **Comments** — `; …` to end of line.
- **Directives** — `define`, `pub define`, `ifdef` / `ifndef` / `else` / `endif`.
- **Data declarations** — the `const` / `u8` / `u16` / `u32` / `u64` keywords.
- **Labels** — `name:` at the start of a line.
- **Pseudo-instructions** — `call`, `ret`, `jump`, `ret_to_bios`, `push`, `pop`.
- **Operations** — the full ZisK op set (`copyb`, `add`, `arith256_mod`, `keccak`,
  `sha256`, `blake2`, `secp256k1_add`, `fcall`, …).
- **Control / modifiers** — `j`, `setpc`, `end`, `sp`.
- **Registers** — `r0`–`r63`.
- **Numbers** — decimal and `0x…` hexadecimal.
- **Operand keywords** — `a`, `c`, `step`.

It is a pure grammar (no build step): the scopes map to standard TextMate scope
names, so whatever color theme you use will style them.

## Install

Pick whichever is convenient. In all cases, open a `.zisk` file afterwards and
confirm the language shows as **"ZisK Assembly"** in the status bar (bottom-right).

### Option 1 — copy (or symlink) into your VS Code extensions folder

The simplest, no tools required. Copy this directory to a folder named
`ziskasm` under your user extensions directory, then restart VS Code:

```bash
# Linux / macOS
cp -r ziskasm/vscode ~/.vscode/extensions/ziskasm
# (or, to keep it in sync with the repo, symlink instead of copy)
ln -s "$(pwd)/ziskasm/vscode" ~/.vscode/extensions/ziskasm
```

Extensions directory by platform:

| Editor / context                    | Path                                  |
|-------------------------------------|---------------------------------------|
| VS Code (local)                     | `~/.vscode/extensions`                |
| VS Code (Windows, local)            | `%USERPROFILE%\.vscode\extensions`    |
| VS Code Insiders (local)            | `~/.vscode-insiders/extensions`       |
| VSCodium (local)                    | `~/.vscode-oss/extensions`            |
| **Remote-SSH / WSL / Dev Container / tunnel** | **`~/.vscode-server/extensions` on the *remote* host** |

> **Editing over Remote-SSH / WSL / a container?** This is the #1 gotcha. The VS
> Code *server* runs on the remote host and loads extensions from
> `~/.vscode-server/extensions` **there** — *not* the `~/.vscode/extensions` on
> your laptop. Copy this folder into `~/.vscode-server/extensions/ziskasm` on the
> machine where the files live (check with **Developer: Show Running Extensions** —
> if your other extensions say "Installed on SSH: …", you're remote).

Restart VS Code (or run **Developer: Reload Window** from the Command Palette).

### Option 2 — package a `.vsix` and install it

Produces a shareable, versioned package. Requires
[`@vscode/vsce`](https://github.com/microsoft/vscode-vsce):

```bash
npm install -g @vscode/vsce
cd ziskasm/vscode
vsce package                       # -> ziskasm-0.1.0.vsix
code --install-extension ziskasm-0.1.0.vsix
```

### Option 3 — run from the Extension Development Host (for hacking on it)

```
code ziskasm/vscode      # open this folder in VS Code
```

Press **F5** ("Run Extension"). A second VS Code window opens with the extension
loaded; open any `.zisk` file there to see the highlighting. Edit the grammar and
reload that window to iterate.

## Files

| File | Role |
|------|------|
| `package.json` | Extension manifest: registers the `ziskasm` language for `.zisk` and its grammar. |
| `language-configuration.json` | Comment (`;`) and bracket behavior. |
| `syntaxes/ziskasm.tmLanguage.json` | The TextMate grammar (the highlighting rules). |

## Keeping the op list current

The **Operations** rule in `syntaxes/ziskasm.tmLanguage.json` lists the ZisK op
mnemonics. If new ops are added to `core/src/zisk_ops.rs`, regenerate the
alternation (longest-first so multi-word mnemonics win):

```bash
grep -oE '\([A-Za-z0-9_]+, "[a-z0-9_]+"' core/src/zisk_ops.rs \
  | grep -oE '"[a-z0-9_]+"' | tr -d '"' | sort -u \
  | grep -vxE 'a|b|i|a32|am32' \
  | awk '{print length, $0}' | sort -rn | awk '{print $2}' | paste -sd'|'
```

and paste the result into the `operation` pattern's `match`.
