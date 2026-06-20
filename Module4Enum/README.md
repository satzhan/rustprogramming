# Module 4 — Enums

Enums and pattern matching, and the file and process operations built on top of them.

**Interactive demo:** [the-tag-decides.html](https://satzhan.github.io/rustprogramming/Module4Enum/the-tag-decides.html) (requires GitHub Pages enabled; otherwise open it rendered via [htmlpreview](https://htmlpreview.github.io/?https://github.com/satzhan/rustprogramming/blob/main/Module4Enum/the-tag-decides.html), which needs no repo settings)

Presentation Guide:: [presentation-checklist.html](https://satzhan.github.io/rustprogramming/Module4Enum/presentation-checklist.html) 

## Files

| File | Topic |
|---|---|
| `01rust-enums-intro-update.md` | Declaration, enums as struct fields, conditional logic, pattern matching, associated data |
| `02rust-enums-importance.md` | Why enums: readability, maintainability, type safety, self-documentation, namespacing |
| `03Assignment-enum-file-operations.md` | Assignment: `Create` / `Rename` file ops driven by an enum and `match` |
| `04interactive-file-operations-rust-assignment.md` | Assignment: menu-driven file ops via `Command::new()` |
| `04raw_basic-systems-programming-rust.md` | Std-only file ops + process management (`Command`, `Stdio` piping) |
| `04rust-html-generator-analysis.md` | HTML generator: `ContentType` enum + structs, written to file |
| `05rust-enum-advanced-examples.md` | Status codes, state machine, config/serde, command parsing, bitflag permissions, events |
| `06rust-llm-prompt-engineering-full.md` | `PromptStrategy` / `QuestionType` enums + `rand` |
| `the-tag-decides.html` | Interactive demo (see below) |

## Demo

Single self-contained HTML page, no build step, no dependencies beyond two web fonts. Six interactive panels, each a working control rather than a picture:

1. One value taking each of an enum's alternatives in turn — the tag made visible.
2. The fixed memory slot: payload sized to the largest alternative plus the tag; `size_of` stays put as the live alternative changes.
3. `match` routing on the tag, with the real `E0004` non-exhaustive error when an alternative is left unhandled.
4. `Option` and `Result` as the same construct; `.unwrap()` as a method, with the real panic on the empty case.
5. Niche optimization: `Option<&T>`, `Option<NonZeroU32>`, and others wrapping for free vs. the tag costing bytes for `Option<i32>`.
6. A shadowing bug: a pattern binding silently overriding a same-named parameter, and the rename that fixes it.

## Scope

Safe Rust throughout. Byte layouts are drawn to show the principle; real offsets and tag positions are compiler-chosen and vary by version and target. Sizes, the compiler error, and the panic text shown on the page were taken from rustc 1.75 on x86-64; `std::mem::size_of` gives exact figures per machine.
