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

std::unique_ptr<OpaqueDoc> create_new_doc(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::string &path);

std::unique_ptr<SharedData> call_classify(const std::unique_ptr<OpaqueCtx> &o_ctx,
                                          const std::unique_ptr<OpaqueDoc> &o_doc, const std::string &obj);

void call_extract(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc,
                  const std::unique_ptr<SharedData> &shared, const std::string &obj);
