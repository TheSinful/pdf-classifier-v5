#pragma once

#include <string>
#include <memory>
#include <mupdf/fitz.h>

struct OpaqueCtx
{
    void *ptr;
};

struct OpaqueDoc
{
    void *ptr;
};

struct SharedData
{
    void *ptr;
};

// typedef void *opaque_ctx;  // fz_context*
// typedef void *opaque_doc;  // fz_document*
// typedef void *shared_data; // any

std::unique_ptr<OpaqueCtx> create_new_ctx(size_t mem_limit);
std::unique_ptr<OpaqueDoc> create_new_doc(const std::string &path, const OpaqueCtx &o_ctx);
std::unique_ptr<SharedData> call_classify(const OpaqueCtx &o_ctx, const OpaqueDoc &o_doc, const std::string &obj);
void call_extract(const OpaqueCtx &o_ctx, const OpaqueDoc &o_doc, const SharedData &shared, const std::string &obj);