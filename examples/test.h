#pragma once

#include <any>
#include <mupdf/fitz.h>

void* classify(fz_context* ctx, fz_document* doc);
void extract(fz_context* ctx, fz_document* doc, void* shared);
