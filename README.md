# FRIDGE Programming Language

FRIDGE is a simple-English programming language. Programs read like plain
instructions:

    remember name as "Hamsan"
    show "Hello " joined with name

    if score is greater than 100
        show "You won!"
    otherwise
        show "Keep playing!"
    end if

FRIDGE is the native application language of the KiterOS ecosystem and is
designed to grow from Hello World to 2D games, 3D games, mobile apps and
desktop apps — all in the same readable English.

## What works in 0.1.0-alpha

| Capability | Status |
|---|---|
| Variables: `remember`, `set`, `becomes`, `add/subtract/multiply/divide/append` | ✅ working |
| English math: `plus`, `minus`, `multiplied by`, `divided by`, `modulo` | ✅ working |
| Conditions: `if` / `otherwise if` / `otherwise` | ✅ working |
| Loops: `repeat N times`, `repeat forever`, `while`, `for each … in …`, `break`, `continue` | ✅ working |
| Functions: `create a function called … taking …`, `return`, recursion, shortcuts `define` | ✅ working |
| Lists: `a list containing …`, `item N of`, `first/last item`, `number of items`, 1-based positions | ✅ working |
| Events: `when the program starts`, `when the program ends` | ✅ working |
| Official shortcuts: `else`, `loop`, `print`, `becomes`, `on start` | ✅ working |
| Syntax highlighting + snippets in VS Code | ✅ working |
| GUI screens/controls/placement | 🔤 grammar ready — runtime arrives in Phase 5 |
| 2D / 3D games | 🔤 grammar reserved — runtime arrives in Phases 6–8 |

## Installing (Windows)

1. Download `fridge-setup-0.1.0-alpha.exe` from the release page.
2. Run the installer. It installs FRIDGE to Program Files, adds it to PATH
   and registers `.frg` files with the FRIDGE icon.
3. **Open a NEW terminal** (PATH changes apply to new terminals) and run:

       fridge version

No desktop shortcut is created, by design.

### Installing the VS Code extension

1. VS Code → Extensions → ⋯ → **Install from VSIX…**
2. Choose `fridge-language-support-0.1.0.vsix` from the release.
3. Open any `.frg` file. Set `fridge.interpreterPath` only if FRIDGE is not on PATH.

## Quick start

Create `hello.frg`:

    remember name as "World"
    show "Hello " joined with name

    repeat 3 times
        show "FRIDGE is working!"
    end repeat

Run it:

    fridge run hello.frg

Check it without running:

    fridge check hello.frg

## CLI

| Command | Meaning |
|---|---|
| `fridge version` | Show version |
| `fridge check main.frg` | Report lexical + grammar problems |
| `fridge check main.frg json` | Machine-readable diagnostics (used by VS Code) |
| `fridge run main.frg` | Execute the program |
| `fridge lex main.frg` | Debug: show tokens |

Exit codes: 0 = clean, 1 = compile diagnostics, 2 = runtime error.

## Examples

See the `examples` folder installed with FRIDGE:

* `hello.frg` — first program (works now)
* `gui_demo.frg` — screen with placed controls (needs Phase 5 runtime)

## Building from source

    # Requires Rust: https://rustup.rs
    cd core
    cargo test
    cargo build --release
    # binary: core/target/release/fridge(.exe)

Windows installer:

    # Requires Inno Setup 6: https://jrsoftware.org/isdl.php
    iscc installer\fridge.iss
    # output: installer\output\fridge-setup-0.1.0-alpha.exe

## Roadmap

M5 formatter + quick fixes → Phase 4 standard library → Phase 5 GUI runtime →
Phase 6 2D games → Phase 8 3D games (WebGPU/WebGL adapter). Status is always
reported honestly in CHANGELOG.md.

## License

MIT — see LICENSE.
