#pragma once

#include <any>
#include <mupdf/fitz.h>
#include <shared/result.h>


Result* classify(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);

// these two are temporary, until i rewrite the python side i cannot cleanly add an unknown variant
// so, to ensure we aren't iterating over an extra useless class i'm making a user-generated one that forces a panic 
Result* UNKNOWN_classify(uint32_t page, fz_context* ctx, fz_document* doc);
Result* UNKNOWN_extract(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);
