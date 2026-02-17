# PDF Classifier v2 - AI Agent Instructions

## What This Is

A **constraint-based, sequential inference engine** for structured documents that separates cheap speculative inference from expensive validation. This is not a generic PDF parser or an AI/ML system — it's a **formal constraint satisfaction system** for sequence labeling under structural rules.

### The Core Problem

Processing a linear stream of pages, assigning each a type from a finite set (chapter, subchapter, diagram, datatable), while:

- Obeying global document structure constraints
- Recovering from incorrect early predictions
- Remaining fast, safe, and explainable
- Using expensive ground-truth validation (PDF classification) sparingly

This is closer to **parsing**, **constraint satisfaction**, and **sequence labeling** than to neural networks.

## Core Architecture: Separation of Responsibilities

### Three-Layer System

The classifier separates concerns into three distinct layers that **must not** be collapsed:

**(A) Inference Layer (cheap, speculative)**

- Runs sequentially on main thread
- Makes guesses about page types based on context + constraints
- Scores candidate types using hard/soft constraints
- Must be fast (<1ms per page) and reversible
- Can be wrong — that's intentional
- Lives in: `src/classifier/mod.rs`, `src/classifier/constraints/`

**(B) Classification/Extraction Layer (expensive, authoritative)**

- Validates an inference guess by calling user-defined C++ functions
- Can fail (returns `UserResult::Fail`)
- Runs off-thread via `WorkerThread` pool
- Produces ground truth that corrects inference
- Feeds back into context for future inferences
- Lives in: `src_cpp/`, `examples/test.cpp` (user code)

**(C) Context Layer (memory, state tracking)**

- Holds structural state (current parent, page history)
- Tracks dynamic statistics (average pair counts, frequencies)
- Marks regions as "trusted" vs "poisoned" (deferred blocks)
- Does NOT decide — it only informs inference
- Lives in: `src/classifier/context.rs`

**Critical Insight**: This separation enables deferred blocks, parallel validation, and graceful degradation when early guesses are wrong.

## Why Three Languages (Orthogonal Constraints)

**Python (`src_py/`)** — Schema Compiler & Build Orchestrator

- **Responsibility**: Schema definition, code generation, build coordination
- **Why Python**: Need reflection and dynamic object graphs for user-facing DSL
- **Not Used For**: Runtime classification (happens entirely in Rust/C++)
- **Key Insight**: Python is the "front-end" that compiles schemas into static artifacts

**C++ (`src_cpp/`)** — PDF Access & Execution Layer

- **Responsibility**: MuPDF integration, user classification logic hosting
- **Why C++**: MuPDF is C-native; page contexts are thread-affine; need predictable ABI
- **Not Used For**: Orchestration, parallelism, or state management
- **Key Insight**: C++ is a _controlled execution environment_, not the engine

**Rust (`src/`)** — Classification Engine & Orchestrator

- **Responsibility**: Parallel scheduling, state machines, algorithmic coordination
- **Why Rust**: Safe parallelism, deterministic ownership, scheduler-heavy workloads
- **Not Used For**: Understanding PDF internals (delegates to C++)
- **Key Insight**: Rust is the "brains" that orchestrates classification across threads

### Responsibility Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│ Python: Schema Definition & Compilation                     │
│ • User defines object hierarchy via def_obj()/def_pair()    │
│ • Generates static artifacts (enums, function tables)       │
│ • Freezes schema at compile time                            │
└────────────────┬────────────────────────────────────────────┘
                 │ (build artifacts)
┌────────────────▼────────────────────────────────────────────┐
│ C++: PDF Execution Layer (no orchestration)                 │
│ • MuPDF context ownership (thread-local)                    │
│ • User classify/extract functions (object-specific logic)   │
│ • Opaque shared state (type-erased user data)               │
└────────────────┬────────────────────────────────────────────┘
                 │ (FFI boundary)
┌────────────────▼────────────────────────────────────────────┐
│ Rust: Classification Orchestrator                           │
│ • Parallel thread pool (owns MuPDF contexts)                │
│ • State machine for page classification                     │
│ • Calls into C++ function tables via cxx FFI                │
│ • Treats shared state as opaque (passes through untouched)  │
└─────────────────────────────────────────────────────────────┘
```

## Developer Workflows

### Building the Project

**IMPORTANT**: The Python builder must run BEFORE Rust compilation:

```bash
# Step 1: Run Python builder (generates headers + builds MuPDF)
cd src_py
python main.py  # Generates build/include/shared/*.h

# Step 2: Build Rust (links against generated artifacts)
cd ..
cargo build
```

The Python [`Builder`](src_py/build.py) class orchestrates:

- MuPDF CMake build → `build/lib/`
- Header generation → `build/include/shared/`
- User project CMake build + validation

### Defining Document Objects (Python DSL)

Objects represent **document-level abstractions** (not PDF primitives). Users define schemas via Python, which compiles them into static artifacts. See [`examples/main.py`](examples/main.py):

```python
from object import ObjectFunc, def_obj, def_pair

# Reference user-defined C++ functions (in test.h)
classify = ObjectFunc("test.h", "obj", "classify")
extract = ObjectFunc("test.h", "obj", "extract")

# Build hierarchy: chapter → subchapter → (diagram, datatable pair)
chapter = def_obj("chapter", classify, extract)
subchapter = def_obj("subchapter", classify, extract, parent=chapter)
(diagram, datatable) = def_pair("diagram", classify, extract,
                                 "datatable", classify, extract,
                                 parent=subchapter)

# Schema compilation: validates functions, generates headers
build = Builder(Path("CMakeLists.txt"), Path("build"))
build.build()
```

**Key Patterns**:

- `def_obj()` creates object types; returns `ReferenceType[Object]` (weakref to avoid cycles)
- `def_pair()` creates mutually-aware pairs (e.g., diagram ↔ datatable relationship)
- Parent-child relationships define classification order (children inherit context)
- `ObjectFunc` links to user C++ functions implementing classification logic

**What Users Actually ImpSchema Compilation Outputs)
The Python builder generates these in `build/include/shared/` — **do not edit manually\*\*:

1. **`generated_page_types.h`**:
    - Enum of all object types (`KnownObject::chapter`, etc.)
    - String conversion functions (`page_type_to_string()`, `page_type_from_string()`)
    - Used by Rust classifier to identify object types at runtime

2. **`reflected_objects.h`**:
    - Serialized object hierarchy as C++ structs (`Node` tree)
    - Encodes parent-child relationships and pairing rules
      -Opaque Shared State Pattern
      Shared state between `classify()` and `extract()` is **intentionally type-erased**:

- Rust/classifier **never interprets** user data (treats as `void*`)
- Avoids templating the entire engine over user types
- User owns lifetime; engine merely forwards pointers
- Originally `std::any`, now raw `void*` for FFI simplicity

**Why**: Classifier is stateless with respect to object contents. It orchestrates, not interprets.

### MuPDF Context Ownership (Parallelism Constraint)

- **256 MiB default limit** per context (see [`STANDARD_MEM_LIMIT`](src/initializer.rs#L4))
- **Thread-local contexts**: MuPDF contexts cannot be shared across threads
- **Upfront initialization**: Context creation is expensive → happens once per worker
- Each Rust worker thread owns one `fz_context*` for its execution lane

**Why This Matters**: Parallel classification requires isolated PDF access per thread. This drives Rust's role as scheduler (manages thread pool) vs C++'s role as executor (owns contexts).

## Constraint System: Hard vs Soft

### Hard Constraints (Structural Impossibilities)

**Purpose**: Prune structurally invalid object types  
**Return**: `bool` (allow/deny)  
**Cannot be violated**: These are absolute rules  
**Performance**: Stateless, compile-time known, inlineable

```rust
pub trait HardConstraint {
    fn eval(ctx: &Context, class: KnownObject) -> bool;
}
```

**Examples**:

- `is_child(parent, child)` — "X cannot be child of Y"
- `is_pair(o1, o2)` — "This pair relationship is invalid"
- Parent-child violations (e.g., chapter cannot be child of diagram)

Hard constraints act as **filters**, not scorers. If any hard constraint returns `false`, that object type is eliminated from consideration.

### Soft Constraints (Preferences, Heuristics)

**Purpose**: Rank remaining candidates after hard filtering  
**Return**: `f32` score (higher = more likely)  
**Can be outweighed**: Preferences, not laws  
**Used for**: Tie-breaking, dynamic weighting, pattern matching

```rust
pub trait SoftConstraint {
    fn eval(ctx: &Context, class: KnownObject) -> f32;
}
```

**Examples** (from `StructuralWeights`):

- `REWARD_only_valid_child` — "This is the only structurally sound child here"
- `REWARD_end_pair` — "This completes an expected diagram-datatable pair"
- `PENALTY_skipped_children` — "This jumps too quickly (subchapter → subchapter)"

Soft constraints form a **linear scoring model** where final score = Σ(constraint_score).

### Why This Design (Performance-Aware)

**Intentional choice**: Constraints are stateless pure functions, not trait objects.

❌ **Rejected approach**: `Vec<Box<dyn Constraint>>`  
✅ **Actual approach**: Named functions called directly

**Benefits**:

- Full inlining (no vtable indirection)
- No allocation, no runtime polymorphism
- Predictable performance in hot inference loop
- Easy auditing (just read the function)

This is a **compiler-style optimization**, not a beginner pattern.

## Structural Knowledge as Lookup Tables

### From Trees to Matrices (O(1) Relationships)

**Problem**: Walking object hierarchy trees is slow for repeated lookups.

**Solution**: Convert schema into adjacency matrices at build time.

**Generated code** (in `src/generated/reflected_objects.rs`):

```rust
type TableMatrix = [[bool; OBJECT_COUNT]; OBJECT_COUNT];

const VALID_CHILDREN: TableMatrix = [
    // Parent: CHAPTER
    [false, true, false, false],   // can have SUBCHAPTER
    // Parent: SUBCHAPTER
    [false, false, true, true],    // can have DIAGRAM, DATATABLE
    // ...
];

#[inline]
pub fn is_child(parent: KnownObject, child: KnownObject) -> bool {
    VALID_CHILDREN[parent as usize][child as usize]
}
```

**What this replaced**: Recursive tree traversal (`obj.children.contains(...)`)  
**Performance gain**: O(n) → O(1) per relationship check  
**Space cost**: `OBJECT_COUNT²` bools (negligible for <64 objects)

### Matrix Types Generated

1. **`VALID_CHILDREN`**: Can Y be a child of X?
2. **`VALID_PARENTS`**: Can X be a parent of Y?
3. **`VALID_PAIRS`**: Do X and Y form a valid pair?

All generated automatically by Python builder's `_build_valid_*_matrix()` methods.

## Inference vs Classification (Critical Distinction)

**DO NOT confuse these terms** — they mean fundamentally different things:

### Inference

- **What**: Guessing page type under uncertainty
- **Speed**: Must be fast (~1ms)
- **Correctness**: Can be wrong
- **Reversibility**: Must not poison system permanently
- **Where**: Rust inference loop (`Classifier::infer()`)

### Classification

- **What**: Verifying guess with ground truth (call user C++ function)
- **Speed**: Expensive (10-100ms, varies by PDF complexity)
- **Correctness**: Authoritative (but can fail with error)
- **When**: After inference, off-thread
- **Where**: C++ classify functions via `WorkerThread::classify()`

**Architectural consequence**: This separation enables:

- Speculative execution (guess 10 pages ahead while 1 validates)
- Parallel validation (classify multiple pages simultaneously)
- Deferred blocks (continue inferring even when validation fails)

## Deferred Blocks: Uncertainty Tracking

### The Problem: Inference Poisoning

In sequential inference, **one wrong decision poisons everything downstream**.

Example:

```
n(5) = inferred as SUBCHAPTER (but actually DIAGRAM)
n(6) = inferred as DIAGRAM (dependent on n(5) being SUBCHAPTER)
n(7) = inferred as DATATABLE (pair with n(6))
```

If `n(5)` classification **fails**, n(6) and n(7) inferences are now garbage.

### The Solution: Defer Blocks

**Instead of preventing poison (impossible), we contain it**:

```rust
pub enum DeferBlock {
    Complete { inner: CompleteDeferBlock },    // start + end known
    Incomplete { inner: IncompleteDeferBlock }, // start known, searching for end
}
```

**What a defer block represents**:

- "We crossed an uncertainty boundary"
- Downstream context is unreliable
- Structural assumptions may be invalid
- Don't trust inferences until we find an anchor

**Lifecycle**:

1. **Detect breaking change**: Classification contradicts inference
2. **Enter defer mode**: `DeferBlock::new(breaking_page)`
3. **Search for independent anchor**: Skip dependent objects, find next subchapter/chapter
4. **Complete block**: `defer.complete(anchor_page)`
5. **Backfill**: Re-infer pages in `[start..end]` with corrected context

### Independent vs Dependent Objects

**Independent objects** (safe anchors):

- Can appear without relying on previous structure
- Can reset/re-anchor context
- End defer blocks
- Examples: `chapter`, `subchapter`

**Dependent objects** (unsafe during defer):

- Only meaningful relative to earlier objects
- Unsafe to infer when context is poisoned
- Examples: `diagram`, `datatable` (paired elements)

**Recovery strategy**: Skip dependent inference inside defer blocks, search only for independents, then backfill.

This is **linear-time recovery**, not exponential backtracking (like parser panic modes).

## Dynamic Weighting (Not Machine Learning)

**This is NOT training a neural network**.

**What it actually is**: Online adaptation via streaming statistics.

### The Model

A **linear scoring model** with streaming parameter updates under hard structural constraints:

```rust
final_score(obj) = Σ hard_constraints(obj) * ∞    // if any false, score = -∞
                 + Σ soft_constraints(obj) * weight
                 + dynamic_adjustments(obj)
```

**Dynamic adjustments** come from classification feedback:

- Average diagram-datatable pair count per subchapter
- Frequency of object types in this document
- Recent pattern matching (last 10 pages)

**Update mechanism**:

```
On classification success:
  - Update context statistics
  - Adjust future soft constraint weights
  - Influence next inferences
```

**Formally**: This is **online learning** in the classical sense, not deep learning.

## Phased Execution Model

The classifier operates in distinct phases with different trust assumptions:

1. **Normal Inference**: Full structural + dynamic weighting
2. **Breaking Detection**: Classification contradicts inference
3. **Deferred Inference**: Conservative mode (structural-only, no dynamic)
4. **Independent Search**: Look only for anchors (chapters, subchapters)
5. **Recovery**: Backfill deferred region with corrected context
6. **Finalize**: Extract data from confidently classified pages

**Key insight**: Correctness is not binary; **confidence is contextual**.

The system prioritizes:

- ✅ Safety (don't crash on bad inferences)
- ✅ Explainability (track why each decision was made)
- ✅ Recoverability (defer blocks, not panics)
- ✅ Graceful degradation (keep going with reduced confidence)

Over:

- ❌ Always being right
- ❌ Always being consistent
- ❌ Inferring everything immediately

This is the mindset difference between **scripts** and **systems**.

## MuPDF Context Ownership (Parallelism Constraint)

### Function Validation System

The [`Builder._validate_expected_funcs_exist()`](src_py/build.py#L169) method enforces compile-time correctness:

1. Parses all `.h` files in user project for function declarations
2. ChBuild Order Dependency\*\* (CRITICAL):
    - **Must run Python builder first**: Generates headers Rust needs
    - Running `cargo build` before `python src_py/main.py` → missing `build/include/shared/*.h`
    - **Why**: Python compiles the schema; Rust links against compiled artifacts

3. **Misunderstanding Language Responsibilities**:
    - Python does **not** participate in runtime classification (it's a schema compiler)
    - C++ does **not** orche (Schema Extension)
4. **Python**: Define in schema: `new_obj = def_obj("name", classify_func, extract_func, parent)`
5. **C++**: Implement functions in user project matching signatures:
    ```cpp
    void* classify_new_obj(fz_context* ctx, fz_document* doc);
    void extract_new_obj(fz_context* ctx, fz_document* doc, void* shared);
    ```
6. **Build**: Run Python builder → regenerates enums/maps → rebuild Rust

**What Changes**:

- `generated_page_types.h` gets new enum variant
- `reflect Strategy
- **Rust tests** ([`src/tests/`](src/tests/mod.rs)): Currently minimal; focus on FFI boundary correctness
- **Python tests** ([`src_py/tests/`](src_py/tests/)): Builder validation (function parsing, header generation)
- **Example project** ([`examples/`](examples/)): Full end-to-end workflow demonstration

**Testing Philosophy**: Schema compiler (Python) catches errors early; runtime (Rust) assumes valid schemas.

## External Dependencies

- **MuPDF**: PDF rendering library (C-native, built via CMake, linked statically)
    - Thread-affine contexts drive parallelism constraints
    - Owns all PDF access (document parsing, page rendering)
- **cxx**: Rust-C++ FFI bridge (handles type conversions, memory safety)
    - Enables Rust orchestration without C++ complexity in scheduler
    - Opaque pointers enforce clear responsibility boundaries
- **CMake 4.2+**: Required for building MuPDF and user projects
    - Python builder orchestrates CMake for both layers

## AI Agent Working Protocol

### Conceptual Understanding Requirement

**Before taking any action**, you must:

1. **Restate the User's Request**: In your own words, explain what you understand the user is asking for. This ensures alignment before proceeding.

2. **Identify the Scope**: Determine if the request involves:
    - **Code Generation** (new functions, classes, or substantial logic)
    - **Conceptual Understanding** (architecture clarification, planning, small edits)
    - **Investigation** (searching, reading, analyzing existing code)

3. **Maintain a Conceptual Log**: Document your reasoning and actions in `.github/agent-conceptual-log.md`

### Conceptual Log Format

Store all AI agent interactions in `.github/agent-conceptual-log.md` with the following structure:

```markdown
## Session: YYYY-MM-DD HH:MM

### User Request

[Verbatim or paraphrased user prompt]

### Agent Understanding

[Restate what you understand the user wants, in your own words]

### Scope Classification

- [ ] Code Generation (functions/classes)
- [ ] Conceptual Work (understanding/planning/small edits)
- [ ] Investigation (reading/searching)

### Actions Taken

[List of concrete actions: file reads, searches, edits]

### Code Changes (if any)

- **File**: `path/to/file`
- **Type**: Function addition | Class creation | Refactoring
- **Purpose**: Why this code was added
- **Functions Added**: `function_name_1()`, `function_name_2()`

### Conceptual Changes (if no code generation)

[Explain what was understood, clarified, or planned. Include:]

- Architecture insights gained
- Responsibility boundaries clarified
- Design decisions understood
- Small edits (< 5 lines, formatting, documentation)

### Reasoning

[Your thought process: why did you approach it this way?]

---
```

### What Counts as "Code Changes" vs "Conceptual"

**Code Changes** (must be logged with function names):

- New function definitions
- New class/struct definitions
- Substantial logic additions (>10 lines)
- FFI boundary extensions
- Build system modifications

**Conceptual Changes** (logged as understanding):

- Documentation updates
- Comment additions
- Variable renames
- Small formatting fixes (<5 lines)
- Investigative file reads
- Architecture discussions
- Planning without implementation

### Mandatory Log Entry Triggers

You **must** append to `.github/agent-conceptual-log.md` when:

1. Any user prompt is received (even if just questions)
2. Any file is edited (code or conceptual)
3. Any investigation yields new understanding
4. No action is taken (explain why not)

**Log First, Act Second**: Write your understanding to the log before making changes.

## AI Agent Guidance

When reasoning about this codebase:

1. **Start with schema definition** ([`examples/main.py`](examples/main.py)) to understand object types
2. **Follow generated artifacts** to see how schemas become code
3. **Trace FFI boundaries** to understand data flow across languages
4. **Recognize parallelism constraints** (thread-local contexts) when considering changes
5. **Respect responsibility boundaries** — don't try to collapse layers

**Not a typical project**: This is schema-driven compilation, not generic PDF parsing. The three languages exist because of orthogonal constraints, not arbitrary choices. Understanding "why each layer exists" is more important than "how each layer works".
**Example Use Cases**:

- Exposing MuPDF metadata extraction
- Adding validation/diagnostics hooks
- Implementing result serialization to Python
    - Always check `ref()` returns non-None before accessing (see `deref()` helper in [`object.py`](src_py/object.py#L13-L16))
    - Parent-child relationships use weakrefs to avoid circular references

**Why Validate in Python**: Catches user errors before expensive C++ compilation. Schema compiler's job is to guarantee valid builds.
}

```

### Generated Artifacts (DO NOT EDIT MANUALLY)
The Python builder generates these in `build/include/shared/`:

1. **`generated_page_types.h`**: Enum of all objects + conversion functions
2. **`reflected_objects.h`**: Serialized object hierarchy as C++ structs
3. **`func_map.h`**: Maps object names → function pointers for dispatch

These are regenerated on every Python build. User code should include them but never modify.

## Project-Specific Conventions

### Memory Management
- **256 MiB default limit** for MuPDF context (see [`STANDARD_MEM_LIMIT`](src/initializer.rs#L4))
- Rust uses `UniquePtr<T>` from cxx for automatic C++ cleanup
- C++ uses opaque void pointers (`OpaqueCtx`, `OpaqueDoc`) to hide MuPDF types from FFI

### Function Validation System
The [`Builder._validate_expected_funcs_exist()`](src_py/build.py#L169) method:
1. Parses all `.h` files in user project for function declarations
2. Checks signatures match:
   - Classify: `void* func(fz_context*, fz_document*)`
   - Extract: `void func(fz_context*, fz_document*, void*)`
3. Raises `RuntimeError` if any expected function is missing/wrong signature

### Build System Integration
- **CMake for C++**: Main [`CMakeLists.txt`](CMakeLists.txt) builds MuPDF integration
- **Cargo for Rust**: [`build.rs`](build.rs) uses `cxx-build` + links against MuPDF libs in `build/lib/`
- **Python orchestrates both**: The `Builder` class runs CMake, then Cargo can link against outputs

## Common Pitfalls

1. **Order Dependency**: Running `cargo build` before Python builder fails (missing headers)
2. **Function Signature Mismatches**: Builder validates at Python build time, not compile time
3. **Weakref Dereference**: Always check `ref()` returns non-None before accessing (see `deref()` helper)
4. **Opaque Type Casting**: C++ must cast `void*` back to `fz_context*`/`fz_document*` (see [`ffi.cpp`](src_cpp/ffi.cpp#L13-L14))

## Integration Points

### Adding New Object Types
1. Define in Python: `new_obj = def_obj("name", classify_func, extract_func, parent)`
2. Implement C++ functions in user project matching expected signatures
3. Run Python builder to regenerate enums/maps
4. Rebuild Rust to link updated artifacts

### Extending FFI Surface
- Add new functions to `#[cxx::bridge]` in [`src/initializer.rs`](src/initializer.rs#L8-L30)
- Implement C++ side in [`src_cpp/ffi.cpp`](src_cpp/ffi.cpp)
- Add corresponding Rust wrapper if needed (see `Context`, `Document` structs)

## Testing
- Rust tests in [`src/tests/`](src/tests/mod.rs) (currently minimal)
- Python tests in [`src_py/tests/`](src_py/tests/) for builder validation
- Example project in [`examples/`](examples/) demonstrates full workflow

## External Dependencies
- **MuPDF**: PDF rendering library (built via CMake, linked statically)
- **cxx**: Rust-C++ FFI bridge (handles type conversions, memory safety)
- **CMake 4.2+**: Required for building MuPDF and user projects
```
