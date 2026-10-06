# Graph Report - pdf-classifier-v5  (2026-10-06)

## Corpus Check
- 140 files · ~60,529 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 8 file(s) not represented in the graph (top: (none) 4, .zip 1, .cmake 1)

## Summary
- 1981 nodes · 4139 edges · 121 communities (77 shown, 44 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 239 edges (avg confidence: 0.88)
- Token cost: 162,077 input · 0 output

## Community Hubs (Navigation)
- Rust Class Serializer
- Visualizer App UI
- C++ Func Map Generator
- C++ FFI Bridge
- Object Description Model
- Hierarchy Serializer
- Page Eval & Prealloc
- Example Classify/Extract Logic
- C++ Class Serializer
- Rust FFI Bindings
- First-Page & Pair Constraints
- Test PDF & Table Model
- Classification/Extraction Jobs
- Ground Truth & Boundary Viz
- Colordoc Classes
- Colordoc PDF Generator
- Python Examples Entry
- Attached Page Wrapper
- Score Lists
- Parsed Func Signatures
- FFI Futures & Responders
- Func Map Serializer Tests
- Inferencer Constraint Counts
- Func Parser Edge-Case Tests
- Build Pipeline Modules
- String Utils Tests
- Visualizer HTTP Server
- Wrapper RAII Tests
- Threading Primitives
- Committed Classifier
- Override Stream Borrow
- Override Constraints
- DataTable Cell Boundaries
- Result Test Traits
- Rust FFI Integration Tests
- Classifier Builder
- Deferral Classifier
- Result Streamer
- Python Override Defs
- BlankAfter Constraint
- Object Builder
- Python Object & Pairs
- Inference Errors & Tracing
- Classifier State Machine
- Member Classify Validation
- Inheritance Parsing Tests
- FzCall Fixture Tests
- Object Lifecycle Tests
- Constraint Design Docs
- Context & Architecture Docs
- PageLock
- Table Test Helpers
- Result Types (C++)
- Object Factory Shims
- FFI User Results
- Known Object Inference
- Capability API
- DataTable Construction
- Expected Subchapter
- Table Cell Types
- Constraint Proc Macro
- Func Map Validator
- Visualizer Model
- Colordoc Class Tests
- Rust Serializer Tests
- PDF Test Fixture
- Colordoc CLI
- Colordoc Fixture
- Chapter Capabilities
- CMake Targets & Deps
- Core CLI Entry
- Committed Classifier Tests
- Text Extraction Tests
- SubChapter Example
- Object Test Fixture
- MuPDF Builder
- FFI Document Lifetimes
- Color Sampling
- Visualizer Docs
- Text & Image Utils
- FzOwned Handle
- Unwrap Macro Tests
- fz_call Wrapper
- C++ Serializer Tests
- Pairing Tests
- Colordoc Palette
- AI Usage Policy
- Text Compression
- Classifier Design Notes
- Chapter Test Fixture
- Diagram Test Fixture
- DataTable Test Fixture
- Legacy Object Form Tests
- Hierarchy Break Constraint
- ColorPage
- Colordoc Result Helpers
- ColorPage Unit Tests
- Chapter Classify Tests
- Test Doc Constants
- Visualizer Log Parser
- User C++ Builder
- ColorPage Fixture Tests
- Hierarchy Tests
- FromU8 Impl Tests
- Drop Tracking Test Utils
- Test Object Stub
- HasPair Method Tests
- FFI String Results
- FzError
- Rust Crates
- Python Package Manifest

## God Nodes (most connected - your core abstractions)
1. `Page` - 118 edges
2. `RustClassSerializer` - 86 edges
3. `UserFuncValidator` - 61 edges
4. `UserFunc` - 57 edges
5. `Context` - 48 edges
6. `Object` - 47 edges
7. `CppFuncMapGenerator` - 45 edges
8. `CppClassSerializer` - 43 edges
9. `DataTable` - 42 edges
10. `ObjectFactory` - 41 edges

## Surprising Connections (you probably didn't know these)
- `Chapter 2 Parts List (sub-chapters 2-1 Engine to 2-16 Accessories)` --semantically_similar_to--> `Chapter`  [INFERRED] [semantically similar]
  data/large_test_doc.pdf → examples/chapter.hpp
- `Exploded-View Figures (full-page JBIG2 scans, e.g. Fig 1 Radius arm and links - front)` --semantically_similar_to--> `Diagram`  [INFERRED] [semantically similar]
  data/large_test_doc.pdf → examples/diagram.hpp
- `Three-level chapter numbering (2 / 2-4 / 2-4-8)` --semantically_similar_to--> `SubChapter`  [INFERRED] [semantically similar]
  data/large_test_doc.pdf → examples/subchapter.hpp
- `Parts List Tables (Fig Item / DMC Army / NATO stock number / Item name / Part No. / No. off / Annotation)` --semantically_similar_to--> `DataTable`  [INFERRED] [semantically similar]
  data/large_test_doc.pdf → examples/table.hpp
- `Live Run Mode (visualizer as TCP frontend)` --semantically_similar_to--> `Stream`  [INFERRED] [semantically similar]
  tools/visualizer/README.md → pdf_classifier_build/src/pdf_classifier/stream.py

## Import Cycles
- 1-file cycle: `pdf_classifier_core/src/inferencer.rs -> pdf_classifier_core/src/inferencer.rs`
- 1-file cycle: `pdf_classifier_core/src/stream.rs -> pdf_classifier_core/src/stream.rs`
- 2-file cycle: `pdf_classifier_core/src/classifiers/committed.rs -> pdf_classifier_core/src/classifiers/mod.rs -> pdf_classifier_core/src/classifiers/committed.rs`
- 2-file cycle: `pdf_classifier_core/src/inferencer.rs -> pdf_classifier_core/src/obj_list.rs -> pdf_classifier_core/src/inferencer.rs`

## Hyperedges (group relationships)
- **Four-Tier Constraint Pipeline Tiers** — github_copilot_instructions_definitive_constraints, github_copilot_instructions_hard_constraints, github_copilot_instructions_soft_constraints, github_copilot_instructions_overrides [EXTRACTED 1.00]
- **Three-Layer Runtime Architecture** — github_copilot_instructions_inference_layer, github_copilot_instructions_classification_extraction_layer, github_copilot_instructions_context_layer [EXTRACTED 1.00]
- **classifier_intermediary Link Contract** — examples_cmakelists_classifier_intermediary, tools_colordoc_cmakelists_classifier_intermediary, pdf_classifier_lib_cmakelists_pdf_classifier_lib, github_copilot_instructions_build_pipeline [INFERRED 0.85]
- **Document Provenance (FOI release of MoD AESP)** — data_large_test_doc_foi2016_00702, data_large_test_doc_illustrated_parts_catalogue, data_large_test_doc_ministry_of_defence, data_large_test_doc_army_equipment_support_publication [INFERRED 0.85]
- **Structures the classifier examples model** — data_large_test_doc_chapter_2_parts_list, data_large_test_doc_three_level_chapter_numbering, data_large_test_doc_parts_list_tables, data_large_test_doc_exploded_view_illustrations [EXTRACTED 1.00]
- **Scanned page + OCR text layers** — data_large_test_doc_pdf_container, data_large_test_doc_scanned_page_images, data_large_test_doc_ocr_text_layer, data_large_test_doc_aesv2_encryption [EXTRACTED 1.00]

## Communities (121 total, 44 thin omitted)

### Community 0 - "Rust Class Serializer"
Cohesion: 0.07
Nodes (8): RustClassSerializer, TestClassEnum, TestFromStrImpl, TestHasChildrenMethod, TestIsFirstInPairMethod, TestIsSecondInPairMethod, TestObjCastErrEnum, TestToStringImpl

### Community 1 - "Visualizer App UI"
Cohesion: 0.11
Nodes (45): assignClassColors(), attachRun(), buildLegend(), buildStrip(), card(), chipEl(), computeAccuracy(), curState() (+37 more)

### Community 2 - "C++ Func Map Generator"
Cohesion: 0.08
Nodes (11): CppFuncMapGenerator, _chapter_funcs(), gen(), _generator(), _multi_funcs(), multi_gen(), TestClassifyFuncMap, TestExtractFuncMap (+3 more)

### Community 3 - "C++ FFI Bridge"
Cohesion: 0.12
Nodes (23): call_classify(), call_extract(), cast_opaque_ctx(), cast_opaque_doc(), cast_opaque_result(), ClonedCtx, clone, create_new_ctx() (+15 more)

### Community 4 - "Object Description Model"
Cohesion: 0.09
Nodes (4): _normalize_class(), UserFuncValidator, TestValidateFuncParamTypes, FuncSyntax

### Community 6 - "Page Eval & Prealloc"
Cohesion: 0.09
Nodes (4): Constructor Pre-allocation Note, Context, Page, ClassifierResultMap

### Community 7 - "Example Classify/Extract Logic"
Cohesion: 0.14
Nodes (11): Diagram, caption, fig_num, evaluate_capabilities_if_any(), ClassificationResult, reason, failing_check(), passing_check() (+3 more)

### Community 8 - "C++ Class Serializer"
Cohesion: 0.12
Nodes (6): CppClassSerializer, TestClassEnum, TestFromStringMethod, TestFullGenerate, TestGuardsAndIncludes, TestToStringMethod

### Community 9 - "Rust FFI Bindings"
Cohesion: 0.15
Nodes (17): call_classify(), call_extract(), classify(), create_new_ctx(), create_new_doc(), Document, drop_ctx(), drop_doc() (+9 more)

### Community 10 - "First-Page & Pair Constraints"
Cohesion: 0.10
Nodes (9): FirstPageRoot, DefinitiveConstraint, SecondInPair, IllegalSecondInPair, HardConstraint, Constraint, FirstInPairConstraint, SoftConstraint (+1 more)

### Community 11 - "Test PDF & Table Model"
Cohesion: 0.08
Nodes (30): AESV2 128-bit Encryption (Standard handler R4), Army Equipment Support Publication (AESP), Chapter 1 General Information, Chapter 2 Parts List (sub-chapters 2-1 Engine to 2-16 Accessories), Chapter 3 Indexes (NSN, part/drawing number, item name, manufacturer code), Contents List, Land Rover Defender XD vehicles: TUL HS, TUM HS, TUM Ambulance HS, Exploded-View Figures (full-page JBIG2 scans, e.g. Fig 1 Radius arm and links - front) (+22 more)

### Community 12 - "Classification/Extraction Jobs"
Cohesion: 0.13
Nodes (5): Classification/Extraction Layer (expensive, authoritative), JobResult, Classification, Extraction, ThreadPool

### Community 13 - "Ground Truth & Boundary Viz"
Cohesion: 0.09
Nodes (9): to_json(), _col_color(), main(), _merge(), align(), detect_anchors(), export_anchor_sequence(), is_table_page() (+1 more)

### Community 14 - "Colordoc Classes"
Cohesion: 0.10
Nodes (8): Caption, ColorObject, color_, expected_, name_, Figure, Section, SubSection

### Community 15 - "Colordoc PDF Generator"
Cohesion: 0.13
Nodes (13): build_arg_parser(), DocSpec, _fill_for(), generate(), GeneratedDoc, main(), paint(), _pair() (+5 more)

### Community 16 - "Python Examples Entry"
Cohesion: 0.11
Nodes (11): Chapter, DataBlock, ExpectedSubChapter, _Object, Serializer, SubChapter, TableData, ExtractionResult (+3 more)

### Community 17 - "Attached Page Wrapper"
Cohesion: 0.10
Nodes (10): Attached, alloc, _ctx, _doc, _page, _page_num, AttachedAllocator, ctx (+2 more)

### Community 18 - "Score Lists"
Cohesion: 0.10
Nodes (8): ScoreList, Score, Custom, Neutral, PUNISHMENT_Heavy, PUNISHMENT_Light, REWARD_Heavy, REWARD_Light

### Community 19 - "Parsed Func Signatures"
Cohesion: 0.13
Nodes (6): ParsedFunc, _classify_func(), _extract_func(), _params(), TestParsedFunc, TestValidateLegacyFunc

### Community 20 - "FFI Futures & Responders"
Cohesion: 0.15
Nodes (6): FFIFuture, WorkerJob, Classify, Extract, WorkerState, WorkerThread

### Community 21 - "Func Map Serializer Tests"
Cohesion: 0.17
Nodes (6): _member_extract(), TestValidate, TestValidateClassifyAndExtractFunc, tmp_cmake(), val(), _validator()

### Community 22 - "Inferencer Constraint Counts"
Cohesion: 0.09
Nodes (7): CastError, ConstraintCastError, InferenceError, ClassCastError, ConstraintCastError, ContextError, ScoreMapMissingConstraint

### Community 28 - "Threading Primitives"
Cohesion: 0.09
Nodes (11): CHANNEL_BUFFER_SIZE, JobType, Classification, Extraction, ThreadError, SendError, AvailablePollCase, Available (+3 more)

### Community 29 - "Committed Classifier"
Cohesion: 0.20
Nodes (4): CommittedClassifier, init_test_committed_classifier(), test_committed_classifier_start(), test_first_page_classification()

### Community 31 - "Override Constraints"
Cohesion: 0.12
Nodes (9): Override, OverrideAction, ClassifyAs, InferAs, Skip, OverrideStream, OverrideStreamExitCase, Exit (+1 more)

### Community 32 - "DataTable Cell Boundaries"
Cohesion: 0.11
Nodes (12): DataTable, cells, column_boundaries, components, height, page_bounds, page_scoped_bounds, pixmap (+4 more)

### Community 33 - "Result Test Traits"
Cohesion: 0.11
Nodes (3): classify_like_engine(), TEST_F(), TEST()

### Community 34 - "Rust FFI Integration Tests"
Cohesion: 0.14
Nodes (7): expect_result(), FIRST_PAGE, make_pool(), test_poll_blocking(), test_poll_draining(), test_worker_classify_call(), test_worker_extract_call()

### Community 35 - "Classifier Builder"
Cohesion: 0.15
Nodes (3): Builder, HeaderCopier, RustModuleGenerator

### Community 36 - "Deferral Classifier"
Cohesion: 0.23
Nodes (5): DeferralClassifier, DeferralExitCase, NoAnchorFound, Successful, test_deferral_run()

### Community 37 - "Result Streamer"
Cohesion: 0.15
Nodes (10): Streamer, StreamerError, FailedToCastPort, FailedToConnectToStream, FailedToFlushStream, FailedToWriteData, FailedToWriteLength, MissingOutputPortEnvVar (+2 more)

### Community 38 - "Python Override Defs"
Cohesion: 0.16
Nodes (6): main(), BlankAfterClassOverride, MultiPageHierarchyBreakOverride, Override, OverrideStream, OverrideSerializer

### Community 39 - "BlankAfter Constraint"
Cohesion: 0.12
Nodes (3): BlankAfter, ContextError, ClassOutOfBounds

### Community 41 - "Python Object & Pairs"
Cohesion: 0.15
Nodes (3): Object, TestFullGenerate, TestObjectCountConst

### Community 42 - "Inference Errors & Tracing"
Cohesion: 0.12
Nodes (8): ClassificationError, ContextRecordError, InferenceError, LoggerInitializationError, PageLockLocked, RecordNotFound, ignore_state_change_if!, STEP_COUNT

### Community 43 - "Classifier State Machine"
Cohesion: 0.22
Nodes (8): Classifier State Machine (Committed / Deferral / OverrideStream), Classifier, ClassifierState, Committed, Deferral, OverrideStream, Transition, Playback Bar

### Community 44 - "Member Classify Validation"
Cohesion: 0.21
Nodes (3): _member_classify(), TestValidateMemberFunc, UserFunc

### Community 46 - "FzCall Fixture Tests"
Cohesion: 0.15
Nodes (10): fake_ctx(), FzCallFixture, ctx, doc, make_tracked(), MakeFixture, OwnedFixture, track_drop() (+2 more)

### Community 48 - "Constraint Design Docs"
Cohesion: 0.17
Nodes (12): Constraints Design Note, ConstraintPriority (proposed), Soft Constraint Budget Note, Dynamic Weights (class-frequency statistics), Inference vs Classification, Context Layer (structural state), Tier 1 Definitive Constraints, Four-Tier Constraint Pipeline (+4 more)

### Community 49 - "Context & Architecture Docs"
Cohesion: 0.22
Nodes (10): Context Design Notes, ContextUpdates (incremental revert), PDF Classifier v5 AI Agent Instructions, Python-Orchestrated Build Pipeline, Generated Rust/C++ Artifacts, Independent vs Dependent Objects, Known Incomplete Areas, ObjectFactory Fluent Schema DSL (+2 more)

### Community 50 - "PageLock"
Cohesion: 0.26
Nodes (7): lock_round_trip_preserves_page(), locked_increment_panics(), PageLock, Locked, Unlocked, try_increment_noops_when_locked(), unlocked_increments()

### Community 51 - "Table Test Helpers"
Cohesion: 0.16
Nodes (4): boundary_debug(), cell_text(), find_cell(), TEST_F()

### Community 52 - "Result Types (C++)"
Cohesion: 0.15
Nodes (5): ExtractionResult, extracted_data, fail_reason, as_string(), contains_text()

### Community 54 - "FFI User Results"
Cohesion: 0.21
Nodes (6): FailUserResult, OkUserResult, UserResult, Fail, Ok, UserResult<T>

### Community 56 - "Capability API"
Cohesion: 0.21
Nodes (7): Capability, CapabilityFailure, capability, reason, TextExtraction, compressed_text, extracted_text

### Community 58 - "Expected Subchapter"
Cohesion: 0.21
Nodes (4): ExpectedSubchapter, path, sub_chapter_num, allocate()

### Community 59 - "Table Cell Types"
Cohesion: 0.15
Nodes (10): TableDataCell, boundary, column, row_num, text, TableDataCountCell, count, TableDataKeyCell (+2 more)

### Community 60 - "Constraint Proc Macro"
Cohesion: 0.20
Nodes (4): ConstraintInput, impl_constraint_enum(), impl_instansiated_constraint_enum(), Variant

### Community 61 - "Func Map Validator"
Cohesion: 0.19
Nodes (3): _normalize_type(), _parse_bases(), _split_params()

### Community 62 - "Visualizer Model"
Cohesion: 0.24
Nodes (14): applyEffects(), buildModel(), emit(), emitEvent(), maxLine(), walk(), foldInfer(), foldOverride() (+6 more)

### Community 63 - "Colordoc Class Tests"
Cohesion: 0.15
Nodes (3): ClassEntry, name, TEST_F()

### Community 64 - "Rust Serializer Tests"
Cohesion: 0.18
Nodes (6): _make_example_objects(), objs(), ser(), _serializer(), TestDefaultImpl, TestIntoU8Impl

### Community 65 - "PDF Test Fixture"
Cohesion: 0.16
Nodes (7): PdfFixture, ctx, doc, TEXT_PAGE, AllocateTest, AttachedTest, DefineObjectTest

### Community 66 - "Colordoc CLI"
Cohesion: 0.21
Nodes (6): build_arg_parser(), collect_results(), define_schema(), main(), report(), run_cpp_tests()

### Community 67 - "Colordoc Fixture"
Cohesion: 0.15
Nodes (5): ColordocFixture, blank_class, ctx, doc, pages

### Community 68 - "Chapter Capabilities"
Cohesion: 0.21
Nodes (3): Chapter, chapter_number, ObjectWith

### Community 69 - "CMake Targets & Deps"
Cohesion: 0.29
Nodes (13): large_test_doc.pdf (test document), classifier_intermediary (examples), classifier_tests (examples gtest), GoogleTest v1.14.0, MuPDF, nlohmann_json v3.12.0, pdf_classifier_lib static library, pdf_classifier_lib_tests (gtest) (+5 more)

### Community 70 - "Core CLI Entry"
Cohesion: 0.21
Nodes (6): Args, main(), init_logger(), init_tests(), def_test_path!, get_root_path!

### Community 72 - "Text Extraction Tests"
Cohesion: 0.18
Nodes (3): run(), StringUtilsPdf, TEST_F()

### Community 74 - "Object Test Fixture"
Cohesion: 0.17
Nodes (4): ObjectFixture, ctx, doc, TEST_F()

### Community 76 - "FFI Document Lifetimes"
Cohesion: 0.17
Nodes (3): Document<'ctx>, drop_result(), OkUserResult<T>

### Community 77 - "Color Sampling"
Cohesion: 0.35
Nodes (3): channel_delta(), rgb_to_string(), within_tolerance()

### Community 78 - "Visualizer Docs"
Cohesion: 0.29
Nodes (9): Extraction Result Streaming over TCP (Rust -> Python), Classifier Run Visualizer README, Error % Tab (ground-truth comparison), Live Run Mode (visualizer as TCP frontend), Margins Tab (soft-score margin diagnostic), Tree Tab (deferral failure branches), Visualizer Single-Page UI (index.html), Detail Tabs (+1 more)

### Community 79 - "Text & Image Utils"
Cohesion: 0.22
Nodes (7): extract_text(), has_image(), PdfText, bbox, font_name, font_size, text

### Community 80 - "FzOwned Handle"
Cohesion: 0.29
Nodes (4): FzOwned, ctx, owned, FzOwnedResource

### Community 84 - "C++ Serializer Tests"
Cohesion: 0.39
Nodes (5): _make_example_objects(), objs(), ser(), _serializer(), tmp_dir()

### Community 86 - "Colordoc Palette"
Cohesion: 0.31
Nodes (4): _const_name(), min_channel_distance(), validate(), write_header()

### Community 87 - "AI Usage Policy"
Cohesion: 0.36
Nodes (7): Project AI Usage Policy, AI as Rubber-Duck (concepts over code-gen), Datatable Visualization (Python), Original Static Classifier, Tracing Instrumentation (AI-assisted), pdf_classifier_lib AI Usage Note, AI as C++ Teacher and Concept Tester

### Community 88 - "Text Compression"
Cohesion: 0.36
Nodes (3): compress_text(), frequency_of(), levenshtein_distance()

### Community 89 - "Classifier Design Notes"
Cohesion: 0.29
Nodes (5): Anchored Approach Note, Classifier Design Notes (main), Breaking vs Non-Breaking Objects, Deferred Block, Panics Note

### Community 90 - "Chapter Test Fixture"
Cohesion: 0.25
Nodes (3): ChapterFixture, ctx, doc

### Community 91 - "Diagram Test Fixture"
Cohesion: 0.25
Nodes (3): DiagramFixture, ctx, doc

### Community 92 - "DataTable Test Fixture"
Cohesion: 0.25
Nodes (3): DataTableFixture, ctx, doc

### Community 95 - "ColorPage"
Cohesion: 0.25
Nodes (5): ColorPage, color, load_error, page_num, sampled

### Community 99 - "Test Doc Constants"
Cohesion: 0.29
Nodes (6): FIRST_CHAPTER_PAGE, FIRST_SUB_CHAPTER_PAGE, LARGE_TEST_DOC_END_PAGE, LARGE_TEST_DOC_START_PAGE, NUM_TEST_THREADS, SECOND_SUB_CHAPTER_PAGE

### Community 100 - "Visualizer Log Parser"
Cohesion: 0.48
Nodes (4): looksLikeFieldList(), parseContent(), parseLog(), splitTopLevel()

### Community 106 - "Drop Tracking Test Utils"
Cohesion: 0.53
Nodes (4): drop_tracked(), new_tracked(), Tracked, id

## Ambiguous Edges - Review These
- `MupdfBuilder` → `cmake_test fixture (install_mupdf)`  [AMBIGUOUS]
  pdf_classifier_build/src/pdf_classifier/tests/cmake/install_mupdf/CMakeLists.txt · relation: conceptually_related_to
- `Context` → `Pre-allocating Known-Size Vecs`  [AMBIGUOUS]
  docs/classifier/constructors.md · relation: conceptually_related_to
- `Python-Orchestrated Build Pipeline` → `pdf_classifier executable (ffi.cpp)`  [AMBIGUOUS]
  pdf_classifier_ffi/CMakeLists.txt · relation: conceptually_related_to

## Knowledge Gaps
- **147 isolated node(s):** `sub_chapter_num`, `path`, `chapter_number`, `fig_num`, `caption` (+142 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 648 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **44 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What is the exact relationship between `MupdfBuilder` and `cmake_test fixture (install_mupdf)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **Why does `Builder` connect `Classifier Builder` to `Rust Class Serializer`, `C++ Func Map Generator`, `Colordoc CLI`, `Object Description Model`, `User C++ Builder`, `Python Override Defs`, `Hierarchy Serializer`, `C++ Class Serializer`, `Python Object & Pairs`, `MuPDF Builder`, `Visualizer Docs`, `Python Examples Entry`, `Context & Architecture Docs`, `Object Factory Shims`, `Build Pipeline Modules`?**
  _High betweenness centrality (0.154) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `Page` (e.g. with `main()` and `test_worker_classify_call()`) actually correct?**
  _`Page` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `sub_chapter_num`, `path`, `chapter_number` to the rest of the system?**
  _147 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Rust Class Serializer` be split into smaller, more focused modules?**
  _Cohesion score 0.07346938775510205 - nodes in this community are weakly interconnected._
- **What is the exact relationship between `Context` and `Pre-allocating Known-Size Vecs`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **Why does `Opaque FFI Boundary (cxx bridge)` connect `Context & Architecture Docs` to `Attached Page Wrapper`, `Chapter Capabilities`, `CMake Targets & Deps`, `FFI User Results`?**
  _High betweenness centrality (0.140) - this node is a cross-community bridge._