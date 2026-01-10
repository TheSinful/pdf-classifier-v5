#pragma once

#include <string>
#include <memory>
#include <mupdf/fitz.h>
#include <shared/result.h>

struct OpaqueCtx
{
    void *ptr; // fz_context*
};

struct OpaqueDoc
{
    void *ptr; // fz_document*
};

struct SharedData
{
    void *ptr; // void* (Result::payload)
};

struct OpaqueResult
{
    void *ptr; // Result*
    ~OpaqueResult() = default;
};

std::unique_ptr<OpaqueCtx> create_new_ctx(size_t mem_limit);

std::unique_ptr<OpaqueDoc> create_new_doc(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::string &path);

inline fz_context *cast_opaque_ctx(const std::unique_ptr<OpaqueCtx> &o_ctx);
inline fz_document *cast_opaque_doc(const std::unique_ptr<OpaqueDoc> &o_doc);

void drop_ctx(const std::unique_ptr<OpaqueCtx> &o_ctx) noexcept;
void drop_doc(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc) noexcept;
void drop_result(const std::unique_ptr<OpaqueResult> &r) noexcept;

std::unique_ptr<SharedData> extract_shared_payload(const std::unique_ptr<OpaqueResult> &r);
const std::string &extract_error_result(const std::unique_ptr<OpaqueResult> &r);
int get_result_status(const std::unique_ptr<OpaqueResult> &r) noexcept;

std::unique_ptr<OpaqueResult> call_classify(const std::unique_ptr<OpaqueCtx> &o_ctx,
                                            const std::unique_ptr<OpaqueDoc> &o_doc, const std::string &obj, uint32_t page);

std::unique_ptr<OpaqueResult> call_extract(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc,
                                           const std::unique_ptr<SharedData> &shared, const std::string &obj, uint32_t page);
