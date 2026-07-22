#pragma once

#include "color_page.hpp"
#include <cstdint>
#include <mupdf/fitz.h>
#include <shared/result.h>

/// The colordoc schema, mirroring the shape of the reference project in
/// `examples/`:
///
///     section                 root, organizational
///     +- subsection           organizational
///        +- figure            dependent, first in pair
///        +- caption           dependent, second in pair
///
/// Every classify function is the same test - "is this page my color?" - so any
/// disagreement with the ground truth emitted by `generate_doc.py` is an engine
/// bug rather than a classification ambiguity.

Result* classify_section(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract_section(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);

Result* classify_subsection(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract_subsection(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);

Result* classify_figure(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract_figure(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);

Result* classify_caption(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract_caption(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);
