---
type: "query"
date: "2026-10-06T16:03:26.819994+00:00"
question: "how do we handle debug outputting from the rust-core classifier"
contributor: "graphify"
outcome: "useful"
source_nodes: ["tracing_subscriber", "Args", "LevelFilter", "Trace-Log Replay (tracing-tree)", "Tracing Instrumentation (AI-assisted)", "parser.js"]
---

# Q: how do we handle debug outputting from the rust-core classifier

## Answer

Expanded from original query via vocab: [tracing, trace, debug, span, level, layer, filter, subscriber, instrumentation, log, info, args]. Graph located the setup in pdf_classifier_core/src/main.rs (tracing_subscriber, HierarchicalLayer, Args->LevelFilter L81) and tests/init.rs, span imports in classifiers (info_span in mod.rs, debug_span in committed.rs, tracing in defer.rs/ovstr.rs/pool.rs), and replay via tools/visualizer (parser.js, Trace-Log Replay). Source confirmed: registry().with(HierarchicalLayer::new(4)).with(EnvFilter 'error,pdf_classifier_core=<level>'); -l/--log_level LevelFilter default info; trace tree to stderr, page->class results to stdout; 32 #[instrument] spans at default info level; tests use HierarchicalLayer::new(2) with no filter. Gap: graph lacks flag names, filter string and stdout/stderr split; Builder.spawn_classifier appends -v which Args does not define.

## Outcome

- Signal: useful

## Source Nodes

- tracing_subscriber
- Args
- LevelFilter
- Trace-Log Replay (tracing-tree)
- Tracing Instrumentation (AI-assisted)
- parser.js