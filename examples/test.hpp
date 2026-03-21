#pragma once

#include <any>
#include <mupdf/fitz.h>
#include <shared/result.h>


Result* classify(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);
