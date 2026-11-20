# CLAUDE.md

Architecture, terminology and invariants live in the Copilot instructions,
imported below so there is one source of truth. If an architectural change makes
that file wrong, say so.

@.github/copilot-instructions.md

---

## Why this repo is different

This project is my personal showpiece for college applications. The ideas and
the engine are mine. AI help is limited and disclosed (see [AIUsage.md](AIUsage.md)).
The rules below exist so the project stays honestly mine. They are not style
preferences.

**Deadline: 2026-12-01.** Goals, in priority order:

1. The test document (`data/large_test_doc.pdf`, pages 44–1277 via
   `python examples/main.py`) classifies **completely and correctly**.
2. `tools/colordoc` builds and runs end-to-end.

Prioritise work that moves one of these forward, and say so when a request doesn't.

## Ownership zones

| Zone | Paths | What Claude does |
|---|---|---|
| **Engine (mine)** | `pdf_classifier_core/src/**` except `tests/` and `test_schema/`; `pdf_classifier_macros/` | Diagnose, explain, review, lay out options. **Never write or edit implementation code here.** |
| **Assisted** | `examples/`, `pdf_classifier_lib/`, `pdf_classifier_ffi/`, `pdf_classifier_build/` | Write code when asked. Lib is sensitive and should be generally wrote with respect to the idea of ease-of-use rather than explicit, efficent implementation. |
| **AI-owned** | `tools/`, all test code (including `pdf_classifier_core/src/tests/`) | Write freely. Log it. |

### Working in the engine zone

"Don't write it" does **not** mean "be vague". Be concrete about the problem:
cite `file:line`, trace the exact control flow, name the bug, and show the
existing code that causes it. Then lay out **every** viable option
("you could try X, Y or Z") with its trade-offs.

**Never leave out an option because it looks like the correct answer.** Holding
back the obvious fix to avoid "solving it for me" is wrong. I want the full
picture, and I decide and write the code myself. Describe changes in words or
by pointing at existing code. Don't hand me replacement code to paste in.

## Settle design decisions before writing code

Before writing code for any non-trivial task, list every design decision it
needs up front: where the code lives, API and signatures, data structures,
ownership/lifetimes, error handling, test approach, and anything that touches
the engine's contracts. Get my answers, then implement **without** stopping to
ask more. If something you couldn't have foreseen comes up mid-task, stop and
ask. Don't guess.

## AI log (`AILog.md`)

Every time Claude writes or edits code, add an entry to [AILog.md](AILog.md) in
the same change. Use the format in that file and keep entries brief.

- Log **only code Claude wrote**: not advice, explanations or reviews, and not
  code I wrote myself, even if we discussed it in conversation.
- If I later rewrite Claude's code substantially, note that on the entry.
- Don't edit the `AIUsage.md` files. They're in my voice. The log is the record.

## Commands

Two machines: Windows desktop (MSVC, multi-config CMake, so always pass
`--config Release` / `-C Release`) and an Arch Linux laptop (GCC). Commands
below are written for the Windows desktop.

```powershell
# Full pipeline: regenerates generated/ + C++ headers, builds MuPDF, user C++ and Rust, runs the classifier
python examples/main.py                         # goal 1: test document
python tools/colordoc/main.py                   # goal 2: colordoc (sets CLASSIFIER_BUILD_DIR itself)
python tools/colordoc/main.py --skip-user-build # also: --build-only, --run-tests, --seed N

# Rust core (links against the installed user build in CLASSIFIER_BUILD_DIR, default examples/build)
cargo check -p pdf_classifier_core
cargo check -p pdf_classifier_core --features no_ffi   # no front-end build needed (link step still fails)
cargo test  -p pdf_classifier_core              # tests compile against src/test_schema/, not generated/

# C++ tests (gtest)
cmake --build examples/build --config Release --target classifier_tests
ctest --test-dir examples/build -C Release
cmake --build tools/colordoc/build --config Release --target colordoc_tests
ctest --test-dir tools/colordoc/build -C Release
cd pdf_classifier_lib; ./test.ps1; ctest --test-dir build -C Release   # standalone lib

# Python front-end
cd pdf_classifier_build; pip install -e .[dev]; pytest
```

Run builds and tests yourself to verify your work. The full `main.py` runs are
slow, so tell me before starting one. Don't commit unless I ask.

## Gotchas

- **`generated/` is never edited by hand.** It's produced by the Python builder,
  and so are the C++ headers in `<build_dir>/include/shared/`. Change the schema
  or the serializers instead.
- **Switching projects:** `examples/` and `tools/colordoc` each install into
  their own build dir. `CLASSIFIER_BUILD_DIR` selects which one Rust links
  against. A stale link shows up at runtime as
  `couldn't find obj: '<class>' in generated func map`.
- Fresh checkout: `cargo build` fails until a `main.py` run has generated
  `generated/` and installed the user build.
- `pdf_classifier_core/src/test_schema/` is **not committed**, but `main.rs`
  loads it under `cfg(test)`. Without it, `cargo test` doesn't compile.
- `graphify-out/` is a generated knowledge graph of the repo. Don't edit it by hand.
- CRLF warnings from git are expected (Windows checkout). Ignore them.

## Conventions

- C++20. Format with clang-format (`examples/.clang-format`: left pointer
  alignment, 125 columns). User objects subclass `ObjectWith<Capabilities...>`
  (e.g. `ObjectWith<TextExtraction>`, see `pdf_classifier_lib/capability.hpp`)
  and are registered with `DEFINE_OBJECT`. MuPDF resources are owned through
  `Attached::allocate`, never raw `fz_drop_*`.
- Rust edition 2024. Cursor moves go through `PageLock`.
- Commit messages: one plain sentence (`Update func_map tests following refactor`).
  Commits containing Claude-written code start with `[AI-written]` and end with
  a `Co-Authored-By` trailer.
