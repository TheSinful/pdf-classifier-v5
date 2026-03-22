#![allow(dead_code)]

// !
// ! Instead of sequentially classifying pages one-by-one, this classifier uses **speculative
// ! branching** to parallelize document understanding across N threads. Each thread has its own
// ! isolated MuPDF context and document, allowing truly parallel page classification.
// !
// !
// ! 1. **Predict**: Look ahead at N pages and estimate what each page will be classified as,
// !    based on learned structural patterns (e.g., "subchapters usually contain 4 diagram-datatable pairs")
// !
// ! 2. **Verify in Parallel**: Spawn N worker threads, each running `classify()` on their predicted
// !    object type. Most predictions succeed immediately.
// !
// ! 3. **Self-Correct**: When a prediction fails (classify returns null), try fallback alternatives
// !    and update the weights that influence future predictions.
// !
// ! 4. **Branch**: When an unexpected parent object appears (e.g., we thought we were still in pairs
// !    but hit a new subchapter), restructure the document tree and re-predict children of the new parent.
// !
// ! ## Example Scenario
// !
// ! Given a schema: `chapter → subchapter → (diagram ↔ datatable pairs)`
// !
// ! After seeing 10 pages past a datatable, we might predict:
// ! ```
// ! Pages 1-8:  diagram → datatable → diagram → datatable (4 pairs under subchapter A)
// ! Page 9:     subchapter (we've learned subchapters average 4 pairs, so expect new parent)
// ! Page 10:    diagram (start of new pair sequence under subchapter B)
// ! ```
// !
// ! **If page 9 fails as subchapter**: Re-classify as diagram, update the weight for "subchapters
// ! typically have 4 pairs" to be less confident, and predict page 10 as datatable (to complete the pair).
// !
// ! **If page 3 succeeds as subchapter but we didn't predict it**: Branch the tree structure.
// ! Pages 4-10 that we thought belonged to the old parent now belong under the new subchapter at page 3.
// !
// !
// ! Traditional sequential classifiers try every object type until one succeeds (expensive).
// ! This classifier **guesses correctly most of the time** by learning patterns, so the expensive
// ! classification work happens in parallel over already-likely-correct predictions.
// ! Maybe utilize a single thread for spawning? So after "cooking" a batch of predictions,
// ! Send them to the "spawner" thread to handle the spawning, while the main thread continues to predict

mod classifier;
mod context;
mod ffi;
mod generated;
mod inferencer;
mod macros;
mod obj_list;
mod page;
mod result_map;
mod score;
#[cfg(test)]
mod tests;

mod threading;
mod weighting;



fn main() {}
