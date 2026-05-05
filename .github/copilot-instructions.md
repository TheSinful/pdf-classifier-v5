# PDF Classifier v5 - AI Agent Instructions

## What This Is

A **constraint-based, sequential inference engine** for structured documents that separates cheap speculative inference from expensive validation. This is not a generic PDF parser or an AI/ML system — it's a **formal constraint satisfaction system** for sequence labeling under structural rules.

### The Core Problem

Processing a linear stream of pages, assigning each a type from a finite set (chapter, subchapter, diagram, datatable), while:

- Obeying global document structure constraints
- Recovering from incorrect early predictions
- Remaining fast, safe, and explainable
- Using expensive ground-truth validation (PDF classification) sparingly

This is closer to **parsing**, **constraint satisfaction**, and **sequence labeling** than to neural networks.

## Repository Layout

The project is a **Rust workspace** with multiple crates plus a Python build package:

```
pdf_classifier_core/    — Main Rust crate: inference engine, state machine, FFI bridge
pdf_classifier_macros/  — Rust proc-macro crate: constraint enum generation
pdf_classifier_ffi/     — C++ FFI layer: ffi.cpp + ffi.hpp (MuPDF glue)
pdf_classifier_build/   — Python package (pdf_classifier): schema compiler & build orchestrator
examples/               — Reference user project (chapter/subchapter/diagram/datatable)
  main.py               — Entry point for running the Python builder
  build/                — CMake + generated headers output directory (default CLASSIFIER_BUILD_DIR)
docs/                   — Developer design notes (informal, not API docs)
```

**Build order** (CRITICAL — do not reverse):
```bash
# Step 1: Python builder — generates C++ headers AND Rust generated/ files
cd examples
python main.py

# Step 2: Cargo — links against artifacts from step 1
cd ..
cargo build
```

`build.rs` reads `CLASSIFIER_BUILD_DIR` env var (default: `../examples/build`) to find MuPDF libs and generated headers.

## Core Architecture: Separation of Responsibilities

### Three-Layer System

The classifier separates concerns into three distinct layers that **must not** be collapsed:

**(A) Inference Layer (cheap, speculative)**

- Runs sequentially on main thread
- Makes guesses about page types based on context + constraints
- Scores candidate types through a four-tier constraint pipeline
- Must be fast (<1ms per page) and reversible
- Can be wrong — that is intentional
- Lives in: `pdf_classifier_core/src/classifier/`, `pdf_classifier_core/src/inferencer.rs`, `pdf_classifier_core/src/constraints/`

**(B) Classification/Extraction Layer (expensive, authoritative)**

- Validates an inference guess by calling user-defined C++ functions via FFI
- Returns `UserResult<Shared>` (classify) or `UserResult<()>` (extract)
- Can fail (returns `UserResult::Fail`)
- Runs off-thread via `WorkerThread` + tokio async channels
- Produces ground truth that corrects inference
- Lives in: `pdf_classifier_ffi/`, user code in `examples/*.cpp`

**(C) Context Layer (memory, state tracking)**

- Holds structural state: `current_parent`, `pages` map, `prev_parents`, `guarantee_failures`
- Does NOT decide — it only informs inference
- `guarantee_failures`: records classes known to fail on specific pages (fed back from deferral)
- Lives in: `pdf_classifier_core/src/context.rs`

**Critical Insight**: This separation enables deferred blocks, parallel validation, and graceful degradation when early guesses are wrong.

## Why Three Languages (Orthogonal Constraints)

**Python (`pdf_classifier_build/`)** — Schema Compiler & Build Orchestrator

- **Responsibility**: Schema definition via `ObjectFactory` DSL, code generation (C++ headers + Rust generated files), build coordination
- **Why Python**: Need reflection and dynamic object graphs for user-facing DSL
- **Not Used For**: Runtime classification (happens entirely in Rust/C++)
- **Key Insight**: Python is the "front-end" that compiles schemas into static artifacts for both C++ and Rust

**C++ (`pdf_classifier_ffi/`)** — PDF Access & Execution Layer

- **Responsibility**: MuPDF integration, dispatching classify/extract calls via generated function maps
- **Why C++**: MuPDF is C-native; page contexts are thread-affine; need predictable ABI
- **Not Used For**: Orchestration, parallelism, or state management
- **Key Insight**: C++ is a controlled execution environment, not the engine

**Rust (`pdf_classifier_core/`)** — Classification Engine & Orchestrator

- **Responsibility**: Parallel scheduling (tokio + threads), state machine, constraint pipeline, context
- **Why Rust**: Safe parallelism, deterministic ownership, scheduler-heavy workloads
- **Not Used For**: Understanding PDF internals (delegates to C++)
- **Key Insight**: Rust is the "brains" that orchestrates classification across threads

### Responsibility Boundaries

```
+-------------------------------------------------------------+
| Python: Schema Definition & Compilation                     |
| * User defines hierarchy via ObjectFactory DSL              |
| * Generates C++ headers (shared/) + Rust generated/ files   |
| * Freezes schema at compile time                            |
+----------------+--------------------------------------------+
                 | (build artifacts)
+----------------v--------------------------------------------+
| C++: PDF Execution Layer (no orchestration)                 |
| * MuPDF context ownership (thread-local)                    |
| * Generated func_map.h for classify/extract dispatch        |
| * OpaqueCtx/OpaqueDoc/SharedData/OpaqueResult wrappers      |
+----------------+--------------------------------------------+
                 | (cxx FFI bridge in ffi.rs)
+----------------v--------------------------------------------+
| Rust: Classification Orchestrator                           |
| * WorkerThread pool (tokio mpsc + OS threads)               |
| * CommittedClassifier / DeferralClassifier state machine    |
| * Four-tier constraint pipeline                             |
| * Treats SharedData as opaque (passes through untouched)    |
+-------------------------------------------------------------+
```

## Python DSL: Defining Document Objects

Objects represent **document-level abstractions** (not PDF primitives). Users define schemas via `ObjectFactory` with a **fluent builder pattern**. See [`examples/main.py`](examples/main.py):

```python
from pdf_classifier import Builder, ObjectFactory, BlankAfterClassOverride
from pathlib import Path

factory = ObjectFactory("test.hpp")  # shared header for generated user includes

factory.new().name("chapter").header("chapter.hpp") \
    .classify("classify_chapter").extract("extract_chapter") \
    .organizational().build()

factory.new().name("subchapter").header("subchapter.hpp") \
    .classify("classify_subchapter").extract("extract_subchapter") \
    .child_of("chapter").organizational().build()

factory.new().name("diagram") \
    .classify("classify").extract("extract") \
    .child_of("subchapter").pair_to("datatable", 1).build()   # pair order 1 = first in pair

factory.new().name("datatable") \
    .classify("classify").extract("extract") \
    .child_of("subchapter").pair_to("diagram", 2).build()     # pair order 2 = second in pair

build = Builder(Path("build"), factory, Path("CMakeLists.txt"))
build.override(BlankAfterClassOverride("chapter")).build()
```

**Key Concepts**:

- `ObjectFactory("test.hpp")` — shared test header that user code implements
- `.organizational()` — marks an object as an **independent anchor** (chapter, subchapter); these can reset/re-anchor context and end defer blocks
- `.pair_to(name, order)` — `order=1` is first in pair (diagram), `order=2` is second (datatable)
- `.child_of(name)` — establishes parent-child structural relationship
- The builder automatically injects an `UNKNOWN` object at discriminant 0 (not user-defined)

**What `.organizational()` means in Rust**: Objects marked organizational become `true` in the generated `INDEPENDENTS` array, which drives `is_independent()` lookups used by deferral and constraint logic.

## Python Builder Generated Artifacts

The `Builder` generates artifacts in two locations — **do not edit manually**:

### C++ headers -> `<build_dir>/include/shared/`

1. **`generated_page_types.h`**: `enum KnownObject`, `page_type_to_string()`, `page_type_from_string()`
2. **`func_map.h`**: `ClassifyFuncMap` and `ExtractFuncMap` (static `vector<Func>` for dispatch)

### Rust files -> `pdf_classifier_core/src/generated/`

1. **`generated_object_types.rs`**: `KnownObject` enum + `OBJECT_COUNT`, `has_children()`, `has_pair()`, `is_first_in_pair()`, `is_second_in_pair()`, `Display`, `TryFrom<u8>`
2. **`reflected_objects.rs`**: `OBJECTS` node tree + `VALID_CHILDREN`/`VALID_PARENTS`/`VALID_PAIRS` adjacency matrices + `INDEPENDENTS` array + helper functions (`is_child`, `is_parent`, `is_pair`, `is_root`, `is_independent`, `get_global_independents`, `get_all_dependents`, etc.)
3. **`overrides.rs`**: `OVERRIDES` constant array of `&'static dyn Override` instances

## Constraint Pipeline (Four Tiers)

The `Inferencer` processes each page through an **ordered four-tier pipeline**:

```rust
// In Inferencer::infer():
candidates.filter_by_definitive_constraints(ctx, page)?  // Tier 1: Definitive — early exit
candidates.filter_by_hard_constraints(ctx, page)?         // Tier 2: Hard — eliminate
candidates.sort_by_soft_constraints(ctx, page)?           // Tier 3: Soft — score + rank
// Tier 4: Overrides — applied by CommittedClassifier after the winner is chosen
```

### Tier 1 — Definitive Constraints

**Purpose**: Conclusively determine the page type — if a definitive constraint fires, inference ends immediately for that page.

**Return**: `bool` — if `true`, this class IS definitively the page type; skip remaining tiers.

**Examples** (in `constraints/definitive/`):
- `FirstPageRoot` — first page of the document is always the root object (e.g. chapter)
- `SecondInPair` — if the previous page was first-in-pair, this page IS second-in-pair (datatable after diagram)

Definitive constraints represent **invariants**, not heuristics.

### Tier 2 — Hard Constraints

**Purpose**: Eliminate structurally impossible candidates.

**Return**: `bool` (PASS/FAIL) — `false` removes the class from consideration permanently for this page.

**Examples** (in `constraints/hard/`):
- `IllegalSecondInPair` — removes second-in-pair classes if the previous page was not first-in-pair

Hard constraints are **filters**.

### Tier 3 — Soft Constraints

**Purpose**: Rank surviving candidates by returning a `Score`.

**Return**: `Score` enum — `REWARD_Heavy(1.0)`, `REWARD_Light(0.5)`, `Neutral(0.0)`, `PUNISHMENT_Light(-0.5)`, `PUNISHMENT_Heavy(-1.0)`, or `Custom(f32)`.

**Soft-max budget**: Constraints are designed to sum to ~1.2-1.5 total, leaving room for future additions.

**Examples** (in `constraints/soft/`):
- `REWARD_IsNaturalChild` — rewards a class that is a valid child of `ctx.current_parent`
- `REWARD_FirstInPair` — rewards first-in-pair class when starting fresh after a completed pair or independent

Soft constraints form a **linear scoring model** where the candidate with the highest total `Score` wins.

### Tier 4 — Overrides

**Purpose**: Handle known structural edge cases that bypass or redirect inference. Applied by `CommittedClassifier._override()` after the inference winner is chosen.

**Return**: `Option<OverrideAction>` where:
- `OverrideAction::Skip` — classify this page as `UNKNOWN` (blank page, no classify call)
- `OverrideAction::InferAs(class)` — override the winner to a specific class (classify call is made)
- `OverrideAction::ClassifyAs(class)` — skip the classify phase entirely, go straight to extract

**Two override abstractions** (in `constraints/overrides/`):

`Override` (single-page, stateless evaluation):
- `BlankAfter { config: KnownObject }` — after page N is inferred as `config`, page N+1 is always `UNKNOWN`. Example: every chapter page is followed by a blank.

`OverrideStream` (multi-page, stateful — design is mature, implementation in progress):
- `MultiPageHierarchyBreak` — when a class X is followed by a known structural pattern before the next anchor (e.g. chapter -> diagrams/datatables until subchapter), skip hierarchy inference entirely. Avoids unnecessary deferral for known document patterns. **Currently contains `todo!()` stubs.**

The `OVERRIDES` array is generated by Python's `OverrideSerializer` from `Override` instances registered via `build.override(...)`.

### Why This Design (Performance-Aware)

Tiers 1-3 use `impl_constraint_enum!` (proc macro in `pdf_classifier_macros`) which generates a **matching enum** with direct dispatch — no vtable indirection, no allocation, full inlining. Tier 4 uses `impl_instansiated_constraint_enum!` for stateful instances (e.g., `BlankAfter` holds a `KnownObject` config field).

## Structural Knowledge as Lookup Tables

Schema relationships are compiled into adjacency matrices at build time in `generated/reflected_objects.rs`:

```rust
type TableMatrix = [[bool; OBJECT_COUNT as usize]; OBJECT_COUNT as usize];

pub const VALID_CHILDREN: TableMatrix  // can Y be a child of X?
pub const VALID_PARENTS: TableMatrix   // can X be a parent of Y?
pub const VALID_PAIRS: TableMatrix     // do X and Y form a valid pair?
pub const INDEPENDENTS: [bool; ...]    // is X an independent (organizational) object?
```

O(1) helper functions: `is_child(parent, child)`, `is_pair(a, b)`, `is_independent(obj)`, `is_root(obj)`, `get_global_independents()`, `get_all_dependents(obj)`.

This replaces recursive tree traversal (`obj.children.contains(...)`) with constant-time matrix lookups.

## Classifier State Machine

The `Classifier` struct drives a two-state machine:

```rust
enum ClassifierState {
    Committed(CommittedClassifier),  // normal sequential inference + classify
    Deferral(DeferralClassifier),    // searching for next independent anchor
    Transition,                       // internal swap sentinel
}
```

### CommittedClassifier

Normal operation. Per `step()`:
1. Run `Inferencer::infer()` on `current_page`
2. Check `OVERRIDES` — may reroute the result
3. Queue classify job on `ThreadPool`
4. Call `ctx.decide()` to record inference
5. `poll()` the thread pool for finished jobs — if a job mismatches inference, set `should_defer = true`

When `should_defer` is true, `Classifier::run()` transitions to `DeferralClassifier`.

### DeferralClassifier

Recovery mode. Entered when a classification result contradicts the inference (a wrong guess was made that corrupts downstream context).

**Phase 1 — Find anchor**:
1. Records `start_page` of the deferral region
2. Cycles through global independents (chapter, subchapter) and spawns classify jobs
3. Failed jobs call `ctx.guarantee_failure_of(class, page)` — future inference knows to avoid that class on that page
4. First successful classify call = anchor found, call `finalize(anchor_page, anchor_class)`

**Phase 2 — Backfill** (`fill_in_dependents()`): Fill pages `[start_page..anchor_page]` using one of three strategies based on the dependent structure of the anchor class:
- `fill_in_when_only_pair(pair)` — alternates the pair (diagram/datatable) across deferred pages when the anchor's only dependents are a single pair
- `fill_in_with_sole_class(class)` — fills all deferred pages with the single dependent class
- `fill_in_by_standard_classification(class)` — runs full inference on deferred pages restricted to the dependent candidates

After backfill, returns a `CommittedClassifier` resuming from the anchor page.

This is **linear-time recovery**: anchor forward, backfill structurally — no exponential backtracking.

### Independent vs Dependent Objects

**Independent** (organizational, safe anchors): `is_independent() == true` — chapter, subchapter. Can appear without relying on previous structure. End defer blocks.

**Dependent**: `is_independent() == false` — diagram, datatable. Only meaningful relative to their parent. Unsafe to infer when context is poisoned by a prior mismatch.

## Dynamic Weighting (Planned — Not Yet Implemented)

The current constraint system uses a **fixed linear scoring model** (Soft constraint tiers with a ~1.5 budget). Dynamic weighting via streaming statistics — average pair counts per subchapter, object-type frequency, recent-page pattern matching — is a **planned future milestone** that will extend `Context` with online statistics.

When implemented, classification feedback will dynamically adjust weights influencing soft constraint scoring, adapting to each document as it is processed.

This is NOT machine learning. It is **online adaptation** via streaming statistics in the classical sense.

## Worker Thread Implementation

Each `WorkerThread` runs in an OS thread (`std::thread::spawn`) with its own isolated MuPDF context and document handle (thread-affine — MuPDF contexts cannot be shared).

Communication uses **tokio mpsc channels** (`CHANNEL_BUFFER_SIZE = 50`) carrying `WorkerJob` variants:
- `WorkerJob::Classify { class, page, responder }` — calls `call_classify()` via FFI
- `WorkerJob::Extract { class, page, shared, responder }` — calls `call_extract()` via FFI, forwarding `SharedData` from the classify result

`ThreadPool` holds `FuturesUnordered` for classify and extract futures separately. `poll()` is called cooperatively (using `noop_waker_ref`) to drain completed futures each loop iteration.

**256 MiB default limit** per MuPDF context (`STANDARD_CTX_MEM_LIMIT` in `ffi.rs`).

## FFI Layer (Rust <-> C++)

The cxx bridge is in [`pdf_classifier_core/src/ffi.rs`](pdf_classifier_core/src/ffi.rs).

**Opaque type wrappers** (C++ side in `ffi.hpp`):
- `OpaqueCtx` -> wraps `fz_context*`
- `OpaqueDoc` -> wraps `fz_document*`
- `SharedData` -> wraps `void*` (user classify payload forwarded to extract)
- `OpaqueResult` -> wraps `Result*` (user function return value)

**Bridge functions**:
- `create_new_ctx(mem_limit)` / `create_new_doc(ctx, path)` — initialization
- `call_classify(ctx, doc, obj_name, page)` — dispatches via `ClassifyFuncMap`
- `call_extract(ctx, doc, shared, obj_name, page)` — dispatches via `ExtractFuncMap`
- `extract_shared_payload(result)` — pulls `SharedData` from an OK classify result (for passing to extract)
- `get_result_status(result)` — 0 = OK, nonzero = Fail

**Rust result types**:
```rust
pub type ClassificationResult = UserResult<Shared>;
pub type ExtractionResult = UserResult<()>;

pub enum UserResult<T> {
    Ok(OkUserResult<T>),   // wraps OpaqueResult + PhantomData<T>
    Fail(FailUserResult),  // wraps OpaqueResult
}
```

`ExtractionResult` payload destination is **not yet designed** — extracted data currently flows nowhere. The planned destination is Python (likely via IPC), since Python is the invoking layer. This is an open design point.

## User C++ Function Signatures

The `UserFuncValidator` checks that user header files declare functions matching these signatures:

- **Classify**: `Result* func_name(uint32_t page, fz_context* ctx, fz_document* doc)`
- **Extract**: `Result* func_name(uint32_t page, fz_context* ctx, fz_document* doc, void* shared)`

Both return `Result*` (not `void*`). The `uint32_t page` is the **first** argument.

## Adding New Object Types

1. **Python**: Register with `ObjectFactory`:
   ```python
   factory.new().name("newtype").header("newtype.hpp") \
       .classify("classify_newtype").extract("extract_newtype") \
       .child_of("parent_name").build()
   ```
2. **C++**: Implement in user header/source matching the required signatures
3. **Build**: Run `python examples/main.py` — regenerates all C++ headers + Rust generated files
4. **Rebuild**: `cargo build` picks up the new `KnownObject` variant and updated matrices

## Project-Specific Conventions

### Constraint Trait Signatures (actual)
```rust
// Definitive + Hard (stateless, no self):
fn eval(ctx: &Context, class: KnownObject, page: Page) -> bool;

// Soft (stateless, no self):
fn eval(ctx: &Context, class: KnownObject, page: Page) -> Score;

// Override (instansiated, takes &self):
fn eval(&self, ctx: &Context, class: KnownObject, page: Page) -> Option<OverrideAction>;

// OverrideStream (instansiated, takes &self):
fn step(&self, ctx: &Context, class: KnownObject, page: Page) -> OverrideStreamStep;
fn should_enter(&self, ctx: &Context, class: KnownObject, page: Page) -> bool;
fn should_exit(&self, ctx: &Context, class: KnownObject, page: Page) -> bool;
```

### Proc-Macro Enum Generation
`pdf_classifier_macros` provides two macros:
- `impl_constraint_enum!(EnumName, ReturnType, VARIANT = StructType, ...)` — stateless constraints (Definitive, Hard, Soft). Generates an enum + `eval()` dispatch + `ENUM_VARIANT_COUNT`.
- `impl_instansiated_constraint_enum!(EnumName, ReturnType, VARIANT = StructType, ...)` — stateful/instansiated constraints (Overrides). Each enum variant holds an instance of its struct.

### Memory Management
- `UniquePtr<T>` from cxx for automatic C++ cleanup on Rust side
- C++ uses opaque void pointer wrappers (`OpaqueCtx`, `OpaqueDoc`) — never expose raw MuPDF types through the FFI boundary

## Build System Integration

- **`pdf_classifier_core/build.rs`**: `cxx-build` compiles `pdf_classifier_ffi/ffi.cpp`, links `libmupdf`, `bindings`, `classifier_intermediary` from `<CLASSIFIER_BUILD_DIR>/lib/`
- **`pdf_classifier_ffi/CMakeLists.txt`**: Builds the C++ intermediary lib
- **`examples/CMakeLists.txt`**: User project — builds example classify/extract functions, links MuPDF
- **Python `Builder`**: Orchestrates MuPDF CMake build, user CMake build+install, C++ header generation, Rust generated file generation

## Common Pitfalls

1. **Build order**: `cargo build` before `python main.py` — missing `generated_object_types.rs`, `reflected_objects.rs`, and C++ headers
2. **Function signature**: Both classify and extract take `uint32_t page` as the first arg and return `Result*`, not `void*`
3. **UNKNOWN is always at index 0**: Python builder injects `UNKNOWN` at discriminant 0 before user objects. `Context::new()` sets `current_parent` to `OBJECTS[1].name` (the first user-defined root, e.g., CHAPTER). Verify this if adding a new root.
4. **Opaque type casting**: C++ side casts `void*` back to `fz_context*` / `fz_document*` via `static_cast` — see `ffi.cpp`
5. **`guarantee_failures` is deferral-only state**: Records failed class/page pairs during deferral for future inference to skip. Not a general inference blacklist.
6. **`OverrideStream` is incomplete**: `MultiPageHierarchyBreak` has `todo!()` stubs — do not attempt to use it until implemented.
7. **ExtractionResult has no consumer**: `extract()` produces `UserResult<()>` — no pipeline exists yet to consume extracted data.

## Testing

- **Rust tests**: `pdf_classifier_core/src/tests/` — `bridge.rs` (FFI smoke tests), `threading.rs`, `thread_pool.rs`, `init.rs` (test doc path helpers)
- **Python tests**: `pdf_classifier_build/src/pdf_classifier/tests/` — builder + serializer validation
- **Integration**: `examples/main.py` demonstrates full end-to-end schema compilation

**Philosophy**: Python builder validates schemas at compile time. Rust assumes valid schemas. Tests focus on FFI boundary correctness and threading behavior.

## External Dependencies

- **MuPDF**: PDF rendering library (C-native, built via CMake, linked statically as `libmupdf`)
- **cxx** (`1.0.x`): Rust-C++ FFI bridge
- **tokio** (`1.x`, features: `sync`, `macros`, `rt`): Async runtime for worker channels
- **futures** (`0.3.x`): `FuturesUnordered` for parallel job collection
- **thiserror**: Structured error types throughout
- **CMake 4.2+**: Required for building MuPDF and user projects
- **pdf_classifier_macros**: Internal proc-macro crate for constraint enum generation

## AI Agent Guidance

When reasoning about this codebase:

1. **Start with `examples/main.py`** — all object types, overrides, and relationships are defined there
2. **Check `pdf_classifier_core/src/generated/`** — this is what Rust actually sees at runtime; it is schema truth
3. **Trace FFI through `ffi.rs`** — Rust calls C++ which dispatches to user functions via `func_map.h`
4. **Respect the constraint tier order**: Definitive -> Hard -> Soft -> Overrides. Do not add scoring to Hard constraints or filtering to Soft ones.
5. **Python generates Rust**: Changes to `ObjectFactory` definitions require re-running `python examples/main.py` before `cargo build`
6. **DeferralClassifier owns backfill**: Deferred region `[start_page..anchor_page]` is filled via `fill_in_dependents()` using one of three strategies based on the dependent structure

**Not a typical project**: Three-language architecture exists because of **orthogonal constraints**, not arbitrary choices. Python = schema reflection, C++ = MuPDF thread affinity, Rust = safe parallelism. Collapsing any two layers violates one of these constraints.
