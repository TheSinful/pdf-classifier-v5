#pragma once

#include <any>
#include <mupdf/fitz.h>
#include <shared/result.h>


Result* classify(fz_context* ctx, fz_document* doc);
Result* extract(fz_context* ctx, fz_document* doc, void* shared);
