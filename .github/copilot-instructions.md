# PDF Classifier v5 — AI Agent Instructions

## What This Is

A **constraint-based, speculative sequence-labeling engine** for structured
documents. It walks a linear stream of PDF pages and assigns each a type from a
finite, user-defined set (in the reference project: `chapter`, `subchapter`,
`diagram`, `datatable`), while:

- obeying global document-structure constraints (hierarchy, pairing, ordering),
- making cheap guesses first and validating expensive ground truth sparingly,
- recovering *linearly* from wrong early guesses instead of backtracking,
- staying fast, safe, and fully explainable.

This is **parsing / constraint satisfaction / sequence labeling**, not machine
learning. There is no model, no training, no inference weights learned from
data. "Inference" here means *constraint-driven deduction*, and the scoring
model is a fixed linear tally. Keep that framing — it is the single most
important thing to understand before touching anything.

### The Core Problem It Solves

Ground-truth classification of a page is **expensive** (it calls user C++ that
pokes at MuPDF). Doing it for every candidate type on every page would be
prohibitive. So the engine *guesses* each page's type from context + structural
constraints (cheap, main-thread, reversible), and only fires the expensive
`classify()` call to confirm the single most likely guess — in parallel, off
the main thread. Most guesses are right. When a guess is wrong, the wrong guess
has poisoned the running structural context, so the engine enters a **deferral**
recovery mode that scans forward to the next safe structural anchor and
backfills the gap structurally. This is linear-time recovery: no exponential
backtracking, no per-page context snapshots.

See [`docs/context.md`](../docs/context.md) for the original design rationale on
why context reversion (snapshot-and-rollback) was rejected in favor of
forward-anchoring deferral.

## Repository Layout

A **Rust workspace** (multiple crates) plus a **Python build package** and a
**C++ FFI layer**:

```
pdf_classifier_core/    Main Rust crate: state machine, inference, constraint
                        pipeline, thread pool, FFI bridge, streaming
pdf_classifier_macros/  Proc-macro crate: constraint-enum generation
pdf_classifier_ffi/     C++ FFI layer: ffi.cpp + ffi.hpp (MuPDF glue)
pdf_classifier_build/   Python package `pdf_classifier`: schema compiler +
                        build orchestrator + result stream frontend
examples/               Reference user project (chapter/subchapter/diagram/
                        datatable); main.py is the end-to-end entry point
tools/visualizer/       Diagnostic web UI that replays a run from its trace log
docs/                   Informal design notes (not API docs)
data/                   Test PDF + static-classifier "ground truth" export
```

## Build & Run Pipeline (Python-Orchestrated)

The Python `Builder` is the front-end that compiles a schema into static
artifacts for **both** C++ and Rust, drives the native builds, then launches
and streams from the classifier. A run of [`examples/main.py`](../examples/main.py)
does the whole thing:

1. Define the schema via `ObjectFactory` (objects, hierarchy, pairing, overrides).
2. `Builder(...).override(...).build()` →
   - builds MuPDF (CMake),
   - generates **Rust** files into `pdf_classifier_core/src/generated/`,
   - generates **C++** headers into `<build_dir>/include/shared/`,
   - validates user C++ function signatures, generates the dispatch func-maps,
   - builds the user C++ project and the Rust core (`cargo build`),
   - returns a `Stream` (a TCP server acting as the result frontend).
3. `await build.spawn_classifier(start, end, threads, doc, verbose)` launches
   `target/debug/pdf_classifier_core.exe` as a subprocess.
4. `async for result in stream.stream_extraction_results()` consumes the
   extraction payloads the running classifier streams back over TCP.

`pdf_classifier_core/build.rs` reads the `CLASSIFIER_BUILD_DIR` env var
(default `../examples/build`) to locate MuPDF libs and generated headers, then
uses `cxx-build` to compile `pdf_classifier_ffi/ffi.cpp` and link
`libmupdf` + the generated bindings.

**Ordering matters**: the Rust `generated/` files and C++ headers must exist
before `cargo build`. Running `python examples/main.py` regenerates them; a bare
`cargo build` against a fresh checkout will fail on missing `generated/` modules.

## Why Three Languages (Orthogonal Constraints)

Each language is present because of a hard constraint, not preference. Do not
collapse layers.

- **Python** (`pdf_classifier_build/`) — schema reflection + build
  orchestration + result frontend. Chosen for dynamic object graphs and
  reflection to power the user-facing DSL. Runs **zero** runtime classification.
- **C++** (`pdf_classifier_ffi/` + user `examples/*.cpp`) — MuPDF access &
  execution. MuPDF is C-native and its contexts are **thread-affine**; C++ gives
  a predictable ABI. It is a controlled execution environment, not an orchestrator.
- **Rust** (`pdf_classifier_core/`) — the engine: state machine, constraint
  pipeline, safe parallel scheduling (tokio + OS threads), context/state.
  It treats PDF internals as opaque and delegates them to C++.

## Runtime Architecture: Three Layers

These are conceptual responsibilities that **must not** be merged:

**(A) Inference layer — cheap, speculative, main-thread.**
Guesses a page's type from context + constraints via the four-tier pipeline.
Fast, reversible, allowed to be wrong. Lives in
[`inferencer.rs`](../pdf_classifier_core/src/inferencer.rs),
[`obj_list.rs`](../pdf_classifier_core/src/obj_list.rs),
[`constraints/`](../pdf_classifier_core/src/constraints/).

**(B) Classification/extraction layer — expensive, authoritative, off-thread.**
Validates a guess by calling user C++ over FFI. `classify()` returns
`UserResult<Shared>` (ground truth + a payload); a successful classify
auto-queues `extract()`, which returns `UserResult<CxxString>` (a JSON payload).
Runs on `WorkerThread`s via tokio channels. Lives in
[`threading/`](../pdf_classifier_core/src/threading/),
[`ffi.rs`](../pdf_classifier_core/src/ffi.rs), and user `examples/*.cpp`.

**(C) Context layer — memory / structural state.**
Holds `current_parent`, the decided `pages` map, `prev_parents`, and
`guarantee_failures`. It **informs** inference; it never decides. Lives in
[`context.rs`](../pdf_classifier_core/src/context.rs).

## The Four-Tier Constraint Pipeline

`Inferencer::infer(ctx, page, candidates)` filters a candidate list through an
ordered pipeline (see [`obj_list.rs`](../pdf_classifier_core/src/obj_list.rs)):

```
Tier 1  filter_by_definitive_constraints  → if one fires, it IS the answer; stop
Tier 2  filter_by_hard_constraints        → eliminate structurally impossible
Tier 3  sort_by_soft_constraints          → score & rank survivors; top wins
Tier 4  overrides                          → applied AFTER, by CommittedClassifier
```

- **Tier 1 — Definitive** (`constraints/definitive/`): invariants, return
  `bool`. A hit ends inference for that page. Examples: `FirstPageRoot` (first
  page is the root class), `SecondInPair` (page after a first-in-pair *is*
  second-in-pair). Trait: `fn eval(ctx, class, page) -> bool`.
- **Tier 2 — Hard** (`constraints/hard/`): filters, return `bool`; `false`
  permanently removes a candidate for this page. Example:
  `IllegalSecondInPair`. Same trait signature as Tier 1.
- **Tier 3 — Soft** (`constraints/soft/`): rankers, return a `Score`
  (`REWARD_Heavy` 1.0 … `PUNISHMENT_Heavy` -1.0, or `Custom(f32)`). Candidate
  with the highest summed score wins. Budget is intentionally ~1.2–1.5 to leave
  room for future constraints. Examples: `REWARD_IsNaturalChild`,
  `REWARD_FirstInPair`. Trait: `fn eval(ctx, class, page) -> Score`.
- **Tier 4 — Overrides**: applied by
  `CommittedClassifier::_override()` after a winner is chosen. Two abstractions
  (see below).

**Do not mix tiers**: no scoring in hard constraints, no filtering in soft ones.

Tiers 1–3 dispatch through `impl_constraint_enum!` (a proc macro in
`pdf_classifier_macros`) which generates a matching enum with direct dispatch —
no vtables, no allocation, full inlining.

### Tier 4 — Overrides (two kinds)

Both are generated by Python into
[`generated/overrides.rs`](../pdf_classifier_core/src/generated/overrides.rs).

**`Override`** — single-page, stateless. `fn eval(&self, ctx, class, page) ->
Option<OverrideAction>`. Registered in the `OVERRIDES` array. The only
implemented one is `BlankAfter { config }`: the page after a page decided as
`config` is forced to `UNKNOWN` (a known blank/record-of-mods page).

**`OverrideStream`** — multi-page, **stateful**, fully implemented. It bypasses
hierarchy inference across a recognized structural run. Trait:
```rust
fn step(&mut self, ctx, page) -> OverrideAction;
fn should_enter(&self, ctx, page) -> bool;
fn should_exit(&self, ctx, page) -> OverrideStreamExitCase;
```
Registered in the `OVERRIDE_STREAMS` array (a `LazyLock<[Mutex<Box<dyn
OverrideStream>>; N]>`, since streams hold mutable state). The implemented one
is `MultiPageHierarchyBreak`: after `chapter`, alternate `diagram`/`datatable`
until a page classifies as `subchapter`, then hand back to committed inference.
This avoids entering deferral for a known-shaped region. It is driven by a
dedicated classifier state (see below).

**`OverrideAction`** variants:
- `Skip` → record `UNKNOWN`, no classify call.
- `InferAs(class)` → override the winner; a classify call still validates it.
- `ClassifyAs(class)` → skip classify, go straight to extract. **Currently
  unused.** Nothing emits it today; wiring extract without a preceding classify
  needs care (extraction depends on the `Shared` payload a classify produces).

## Structural Knowledge as Lookup Tables

Schema relationships are compiled into `O(1)` adjacency matrices at build time
in [`generated/reflected_objects.rs`](../pdf_classifier_core/src/generated/reflected_objects.rs):

```rust
pub const VALID_CHILDREN: TableMatrix   // can Y be a child of X?
pub const VALID_PARENTS:  TableMatrix   // can X be a parent of Y?
pub const VALID_PAIRS:    TableMatrix   // do X and Y form a pair?
pub const INDEPENDENTS: [bool; OBJECT_COUNT]  // is X organizational?
```

Helper fns: `is_child`, `is_parent`, `is_pair`, `is_root`, `is_independent`,
`get_global_independents`, `has_dependents`, `get_all_dependents`. These replace
recursive tree walks with constant-time matrix lookups. The `OBJECTS` constant
is the root node **tree** (in the reference schema a single-element array whose
one node is `CHAPTER`); do not confuse `OBJECTS` indexing with `KnownObject`
discriminants (where `UNKNOWN` is 0).

### Independent vs Dependent Objects

- **Independent** (organizational, `is_independent() == true`): e.g. `chapter`,
  `subchapter`. Safe structural anchors — meaningful without prior context, can
  re-anchor `current_parent`, and terminate deferral/override-stream regions.
- **Dependent** (`is_independent() == false`): e.g. `diagram`, `datatable`.
  Only meaningful relative to a parent; unsafe to infer once context is poisoned.

## Classifier State Machine

[`classifiers/mod.rs`](../pdf_classifier_core/src/classifiers/mod.rs) drives a
**three-state** machine (plus a `Transition` swap sentinel):

```rust
enum ClassifierState {
    Committed(CommittedClassifier),        // normal sequential inference + classify
    Deferral(DeferralClassifier),          // recovery: scan for the next anchor
    OverrideStream(OverrideStreamClassifier), // stream a known structural run
    Transition,                            // internal ownership-swap sentinel only
}
```

`Classifier::run()` loops: it replaces `self.state` with `Transition` (to move
ownership out), inspects the committed classifier's flags, and either steps in
place, schedules a transition, or exits when the cursor reaches `end_page`.
`DeferralClassifier` and `OverrideStreamClassifier` both wrap a
`CommittedClassifier` as their `base`, so shared behavior lives in one place.

### CommittedClassifier — normal operation

Per [`step()`](../pdf_classifier_core/src/classifiers/committed.rs):
1. `Inferencer::infer()` the current page.
2. Consult `OVERRIDES` — may reroute via `handle_override`.
3. Queue a `classify` job on the `ThreadPool`.
4. `try_decide_as()` → record the decision in `Context` and advance the cursor.
5. `poll()` the pool; a **failed** classify result sets `should_defer = true`.

When `should_defer` is set, `run()` transitions to `DeferralClassifier`.
(Note: deferral triggers on classify **failure**, not on a value mismatch — the
engine only ever classifies its single top guess.)

### DeferralClassifier — recovery

Entered when a classify result contradicts the guess (context is now poisoned).
Phase 1, `find_next_independent()`: scan forward, spawning `classify_unchecked`
probes for the global independents; failed probes call
`ctx.guarantee_failure_of(class, page)` so future inference skips that class on
that page; the first successful probe is the **anchor**, and `finalize()` records
it. Phase 2, `fill_in_dependents()`: backfill the deferred region
`[start_page .. anchor_page)` with one of three strategies keyed on the anchor's
dependents:
- `fill_in_with_only_pair` — alternate the pair across the gap,
- `fill_in_with_sole_class` — one dependent class fills everything,
- `fill_in_by_standard_classification` — re-run inference restricted to the
  anchor's dependents.

Then it returns a `CommittedClassifier` resuming at the anchor.

### OverrideStreamClassifier — streamed run

Entered when `should_enter_override_stream()` matches. `till_stream_end()` loops
calling the stream's `step()` (which decides + queues each page) and checking
`should_exit`; on the exit class classifying successfully it records that page
and hands control back to committed inference.

## Page Cursor Discipline (`PageLock`)

Page advancement is enforced by a type, not a loose integer or boolean flag —
see [`page_lock.rs`](../pdf_classifier_core/src/page_lock.rs). This exists
because historically the worst bugs came from advancing the cursor in the wrong
place (double-advances, skips).

```rust
enum PageLock { Unlocked(Page), Locked(Page) }
```

- `increment()` / `increment_by(by)` — advance; **panic** on a `Locked` cursor.
  Use where advancing is mandatory.
- `try_increment_by(by)` — advance if unlocked; a **no-op** returning `None` if
  locked. Use where standing still is legitimate.
- `lock()` / `unlock()` — transition state, preserving the page.

The two decision entry points on `CommittedClassifier` reflect this split:
- `decide_as(class, page)` records and **must** advance (`increment_by`, panics
  if locked).
- `try_decide_as(class, page)` records and advances *if unlocked*; if locked it
  returns `Err(ClassificationError::PageLockLocked)`.

Deferral relies on this: it `lock()`s the base's cursor and keeps its own
separate unlocked scan cursor, so backfill decisions record into `Context`
without moving the committed cursor. When it finalizes, it unlocks the base at
the anchor and advances once.

## Worker Threads & Thread Pool

Each `WorkerThread` ([`threading/mod.rs`](../pdf_classifier_core/src/threading/mod.rs))
owns an OS thread with its **own** isolated MuPDF `fz_context` + document
(thread-affine — MuPDF contexts cannot cross threads). Communication is via
tokio mpsc channels (`CHANNEL_BUFFER_SIZE = 50`) carrying `WorkerJob::Classify`
/ `WorkerJob::Extract`. Default per-context limit is 256 MiB
(`STANDARD_CTX_MEM_LIMIT`).

The `ThreadPool` ([`threading/pool.rs`](../pdf_classifier_core/src/threading/pool.rs))
holds `FuturesUnordered` for classify and extract futures and is `poll()`ed
cooperatively (via `noop_waker_ref`). Two behaviors to know:
- A **successful classify auto-queues its extraction** — the classify's `Shared`
  payload is stashed in `pending_extract_shared`, keyed by page.
- Extraction is **worker-affine**: the `Shared` payload holds MuPDF objects tied
  to the classifying worker's `fz_context`, so the extract job is dispatched
  back to that same worker; if it is momentarily busy the job is re-queued.

## FFI Boundary (Rust ↔ C++)

The `cxx` bridge is in [`ffi.rs`](../pdf_classifier_core/src/ffi.rs). Never
expose raw MuPDF types across the boundary — everything is an opaque wrapper:
`OpaqueCtx` (`fz_context*`), `OpaqueDoc` (`fz_document*`), `SharedData` (the
classify payload forwarded to extract), `OpaqueResult` (`Result*`).

Result types:
```rust
pub type ClassificationResult = UserResult<Shared>;      // classify
pub type ExtractionResult     = UserResult<CxxString>;   // extract (JSON string)

pub enum UserResult<T> { Ok(OkUserResult<T>), Fail(FailUserResult) }
```

User C++ function signatures (validated by `UserFuncValidator`):
- **Classify**: `Result* fn(uint32_t page, fz_context* ctx, fz_document* doc)`
- **Extract**:  `Result* fn(uint32_t page, fz_context* ctx, fz_document* doc, void* shared)`

`page` is always first; both return `Result*` (not `void*`).

## Extraction Result Streaming (Rust → Python)

Extraction output is fully wired end-to-end:
`extract()` returns a JSON string → Rust `Streamer`
([`stream.rs`](../pdf_classifier_core/src/stream.rs)) writes framed
`(page, class, payload)` bytes over a TCP socket to `CLASSIFIER_OUTPUT_PORT` →
Python's `Stream` ([`pdf_classifier_build/.../stream.py`](../pdf_classifier_build/src/pdf_classifier/stream.py))
accepts the connection and yields `ExtractionResult`s from
`stream_extraction_results()`. The binary **refuses to start** without
`CLASSIFIER_OUTPUT_PORT` set, because a frontend is mandatory. `Streamer` is a
plain struct with `send_data(page, class, &[u8])`.

## Python DSL: Defining Document Objects

Objects are document-level abstractions, built with a fluent
`ObjectFactory`. From [`examples/main.py`](../examples/main.py):

```python
factory = ObjectFactory("test.hpp")  # shared header the user code implements

factory.new().name("chapter").header("chapter.hpp") \
    .classify("classify_chapter").extract("extract_chapter") \
    .organizational().build()
factory.new().name("subchapter").header("subchapter.hpp") \
    .classify("classify_subchapter").extract("extract_subchapter") \
    .child_of("chapter").organizational().build()
factory.new().name("diagram").header("diagram.hpp") \
    .classify("classify_diagram").extract("extract_diagram") \
    .child_of("subchapter").pair_to("datatable", 1).build()   # 1 = first in pair
factory.new().name("datatable").header("table.hpp") \
    .classify("classify_datatable").extract("extract_datatable") \
    .child_of("subchapter").pair_to("diagram", 2).build()     # 2 = second in pair

build = Builder(examples_root / "build", factory, examples_root / "CMakeLists.txt")
build.override(BlankAfterClassOverride("chapter"))
build.override(MultiPageHierarchyBreakOverride("chapter", True, "subchapter",
                                               ["diagram", "datatable"]))
stream = build.build(skip_user_build=True)
```

- `.organizational()` → sets `true` in the generated `INDEPENDENTS` array
  (drives `is_independent()`).
- `.pair_to(name, order)` → `order=1` first in pair, `order=2` second.
- `.child_of(name)` → parent/child relationship.
- The builder injects an `UNKNOWN` object at discriminant 0 (not user-defined).

## Generated Artifacts (do not edit by hand)

**Rust** → `pdf_classifier_core/src/generated/`:
- `generated_object_types.rs` — `KnownObject` enum, `OBJECT_COUNT`,
  `has_children/has_pair/is_first_in_pair/is_second_in_pair`, `Display`,
  `TryFrom<u8>`.
- `reflected_objects.rs` — `OBJECTS` node tree, the adjacency matrices, the
  `INDEPENDENTS` array, and the helper fns above.
- `overrides.rs` — the `OVERRIDES` array and the `OVERRIDE_STREAMS` `LazyLock`.

**C++** → `<build_dir>/include/shared/`:
- `generated_page_types.h` — `enum KnownObject`, `page_type_to_string`,
  `page_type_from_string`.
- `func_map.h` — `ClassifyFuncMap` / `ExtractFuncMap` for dispatch.

## Adding a New Object Type

1. **Python**: register it with `ObjectFactory` (`.name().header().classify()
   .extract().child_of()...`).
2. **C++**: implement the classify/extract functions with the required
   signatures in the user header/source.
3. **Rebuild**: `python examples/main.py` regenerates all artifacts and rebuilds
   both native sides.

## Proc-Macro / Constraint Conventions

`pdf_classifier_macros` provides:
- `impl_constraint_enum!(Name, Ret, VARIANT = Struct, ...)` — stateless
  constraints (Definitive/Hard/Soft). Generates a matching enum, `eval()`
  dispatch, and `ENUM_VARIANT_COUNT`.
- `impl_instansiated_constraint_enum!(Name, Ret, VARIANT = Struct, ...)` —
  stateful/instantiated constraints (Overrides hold config, e.g. `BlankAfter`).

Memory management across the boundary uses `cxx`'s `UniquePtr<T>` for automatic
C++-side cleanup; opaque void wrappers keep MuPDF types off the Rust side.

## Diagnostic Visualizer (`tools/visualizer/`)

A stdlib-Python-served, self-contained web UI that **replays a run purely from
its tracing-tree log** — no changes to the classifier. It renders the page strip,
a step-by-step stepper (inference tiers, overrides, deferral probes, backfill),
a document tree, inference margins, state/errors/extractions, and an **error %
tab**. See its [README](../tools/visualizer/README.md).

The error % tab compares a run's decisions against a per-page **ground truth** in
`static/groundtruth.js`, generated by `build_groundtruth.py` from the static
export in `data/large_test_doc_classified/` combined with the PDF's own text
signatures (the export gives the authoritative anchor *sequence*; the PDF text
positions every page). Regenerate it with `python build_groundtruth.py`
(needs `pymupdf`) if the export or page range changes.

## Known Incomplete / In-Progress Areas

Be aware these are not finished; verify current code before relying on them:

- **Deferral is not yet robust.** The anchor scan probes one independent class
  per page before advancing, so a real anchor can be walked past; backfill pair
  parity is seeded from the region start without checking the last committed
  decision. This is the actively-worked area (`page-keeper` / issue #14 lineage).
- **`OverrideAction::ClassifyAs` is unused** and would need extraction-without-
  classify support to work.
- **Context has no rollback.** `Context::decide` records unconditionally; the
  `todo` about reverting an incorrect independent decision is unimplemented —
  deferral is the recovery mechanism instead.
- Some generated helpers (e.g. `get_all_independents`) are only exercised by the
  reference schema and may not generalize; check them if you add object types.

## AI Agent Guidance

1. **Start at [`examples/main.py`](../examples/main.py)** — the schema,
   overrides, and page range are all defined there.
2. **Trust [`generated/`](../pdf_classifier_core/src/generated/) as schema
   truth** — it is what Rust actually compiles against at runtime.
3. **Respect the tier order** Definitive → Hard → Soft → Overrides; never add
   scoring to hard constraints or filtering to soft ones.
4. **Python generates the native code.** Any change to `ObjectFactory`
   definitions requires re-running `python examples/main.py` before `cargo build`.
5. **Cursor moves go through `PageLock`.** Use `try_decide_as` where standing
   still may be valid (deferral), `decide_as` where advancing is mandatory.
   Never mutate a raw page counter.
6. **Deferral owns recovery**, not context rollback. The deferred region
   `[start_page .. anchor_page)` is backfilled structurally.
7. **This is not ML.** If a change starts to look like learned weights or
   training, stop — the design is a fixed linear scoring model plus online
   statistics *planned* for the future, in the classical streaming sense only.

**Not a typical project**: the three-language split exists because of orthogonal
constraints — Python for schema reflection, C++ for MuPDF thread affinity, Rust
for safe parallelism. Collapsing any two violates one of those constraints.
