# Module 3 — Structs

Structs, struct memory layout, and the system calls underneath file I/O and process spawning.

**Interactive demo:** [the-machine-underneath.html](https://satzhan.github.io/rustprogramming/Module3Struct/the-machine-underneath.html) (requires GitHub Pages enabled; otherwise view via [htmlpreview](https://htmlpreview.github.io/?https://github.com/satzhan/rustprogramming/blob/main/Module3Struct/the-machine-underneath.html))

## Files

| File | Topic |
|---|---|
| `01comprehensive-rust-guide-llm-agent.md` | Structs: definition, methods, tuple/unit structs, update syntax, `match` patterns, layout, nesting, ownership-taking methods, associated constants |
| `02struct-memory-layout.md` | `size_of` / `align_of`, padding, field ordering, vs. C++ |
| `03-1what-is-systemcall.md` | System calls: user vs. kernel mode, trap table, mode switch |
| `03-2rust-syscall-example.md` | `write` / `exit` via inline assembly |
| `03-3rust-file-operations.md` | File I/O: create, read, append, paths, metadata |
| `04EasyAssignmentrust-structs-IOoperations.md` | Assignment: struct + console/file I/O |
| `05MediumAssignmentust-struct-file-io-assignment.md` | Assignment: Book Catalog, struct ↔ file serialization |
| `06rust-execute-commands.md` | `std::process::Command` |
| `07rust-python-integration.md` | Rust writes a summary; Python reads it and renders a chart |
| `08rust-python-execution-agent.md` | `PythonAgent` struct for running Python scripts |
| `09UpperMediumAssignmentlinux-command-simulator-rust-agent.md` | Assignment: command simulator — struct holding state, running commands, persisting history |

## Demo contents

Single self-contained HTML page, no build step, no dependencies beyond two web fonts. Six interactive diagrams of mechanisms the files use but don't fully explain:

1. `String` layout — stack handle (ptr/len/cap, 24 bytes) vs. heap allocation; move vs. clone.
2. Struct packing — alignment and padding; `repr(Rust)` reorders fields, `#[repr(C)]` doesn't.
3. The user/kernel boundary a system call crosses, stepped stage by stage.
4. Buffered I/O as system-call amortization, shown with a call counter.
5. `fork` / `exec` on `Command::spawn`, with a file as the channel between processes.
6. `&self` / `&mut self` / `self` and their effect on ownership.

## Scope

Safe Rust except for the inline-assembly system call. Lifetimes, manual heap allocation, and `unsafe` beyond that call are out of scope. Diagrams simplify byte offsets, syscall numbers, and process behavior, all of which vary by platform and compiler version; `std::mem::size_of` and a debugger give exact values per machine.
