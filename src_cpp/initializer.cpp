#include "initializer.h"
#include <shared/generated_page_types.h>
#include <shared/func_map.h>
#include <filesystem>
#include <format>
#include <iostream>

#define THROW_MUPDF_ERROR(msg)                                                   \
    do                                                                           \
    {                                                                            \
        std::string __mupdf_error_msg = fz_caught_message(ctx);                  \
        throw std::runtime_error(std::format("{} {}", #msg, __mupdf_error_msg)); \
    } while (0);

std::unique_ptr<OpaqueCtx> create_new_ctx(size_t mem_limit)
{
    fz_context *ctx = fz_new_context(NULL, NULL, mem_limit);

    if (!ctx)
    {
        throw std::runtime_error("Failed to create context!");
    }

    return std::make_unique<OpaqueCtx>(ctx);
}

std::unique_ptr<OpaqueDoc> create_new_doc(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::string &doc_path)
{
    if (!std::filesystem::exists(doc_path))
    {
        throw std::runtime_error("File doesn't exist at path provided " + doc_path);
    }

    fz_context *ctx = static_cast<fz_context *>(o_ctx->ptr);

    if (!ctx)
    {
        throw std::runtime_error("Failed to access created context!");
    }

    fz_try(ctx)
    {
        fz_register_document_handlers(ctx);
    }
    fz_catch(ctx)
    {
        THROW_MUPDF_ERROR("Failed to register document handlers!");
    }

    fz_document *doc;
    fz_try(ctx)
    {
        doc = fz_open_document(ctx, doc_path.c_str());
        break;
    }
    fz_catch(ctx)
    {
        THROW_MUPDF_ERROR("Failed to create document!");
    }

    return std::make_unique<OpaqueDoc>(doc);
}

std::unique_ptr<OpaqueResult> call_classify(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc, const std::string &obj)
{
    fz_context *ctx = static_cast<fz_context *>(o_ctx->ptr);
    fz_document *doc = static_cast<fz_document *>(o_doc->ptr);

    Func *found_func = nullptr;
    for (int i = 0; i < ClassifyFuncMap.size(); i++)
    {
        Func func = ClassifyFuncMap[i];

        if (func.obj_name == obj)
        {
            found_func = &func;
        }
    }

    if (found_func == nullptr)
    {
        throw std::runtime_error("couldn't find obj: " + obj + "in generated func map!");
    }

    void *ptr = found_func->ptr;
    if (ptr)
    {
        typedef Result *(*classify_func)(fz_context *, fz_document *);
        classify_func fn = reinterpret_cast<classify_func>(ptr);
        Result *res = fn(ctx, doc);

        if (!res)
        {
            throw std::runtime_error("classify returned null for " + obj);
        }

        return std::make_unique<OpaqueResult>(res);
    }
    else
    {
        throw std::runtime_error("couldn't find classify func ptr " + obj);
    }
}

std::unique_ptr<OpaqueResult> call_extract(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc, const std::unique_ptr<SharedData> &shared, const std::string &obj)
{
    fz_context *ctx = static_cast<fz_context *>(o_ctx->ptr);
    fz_document *doc = static_cast<fz_document *>(o_doc->ptr);

    Func *found_func = nullptr;
    for (int i = 0; i < ClassifyFuncMap.size(); i++)
    {
        Func func = ExtractFuncMap[i];

        if (func.obj_name == obj)
        {
            found_func = &func;
        }
    }

    if (found_func == nullptr)
    {
        throw std::runtime_error("couldn't find obj: " + obj + "in generated func map!");
    }

    void *ptr = found_func->ptr;
    if (ptr)
    {
        typedef Result *(*extract_func)(fz_context *, fz_document *, void *);
        extract_func fn = reinterpret_cast<extract_func>(ptr);
        Result *res = fn(ctx, doc, shared->ptr);
        return std::make_unique<OpaqueResult>(res);
    }
    else
    {
        throw std::runtime_error("couldn't find classify func ptr " + obj);
    }
}

inline fz_context *cast_opaque_ctx(const std::unique_ptr<OpaqueCtx> &o_ctx)
{
    return reinterpret_cast<fz_context *>(o_ctx->ptr);
}

inline fz_document *cast_opaque_doc(const std::unique_ptr<OpaqueDoc> &o_doc)
{
    return reinterpret_cast<fz_document *>(o_doc->ptr);
}

void drop_ctx(const std::unique_ptr<OpaqueCtx> &o_ctx) noexcept
{
    fz_drop_context(cast_opaque_ctx(o_ctx));
}

void drop_doc(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc) noexcept
{
    fz_drop_document(cast_opaque_ctx(o_ctx), cast_opaque_doc(o_doc));
}

void drop_result(const std::unique_ptr<OpaqueResult> &r) noexcept
{
    if (!r)
        return;

    Result *inner = reinterpret_cast<Result *>(r->ptr);
    if (inner->type == Result::Type::OK && inner->deleter)
    {
        inner->deleter(inner->payload);
    }

    // inner->fail_rsn is implicitly destructed via delete
    delete inner;
}

std::unique_ptr<SharedData> extract_shared_payload(const std::unique_ptr<OpaqueResult> &r)
{
    Result *inner = reinterpret_cast<Result *>(r->ptr);
    if (inner->type != Result::OK)
    {
        throw std::runtime_error("Attempted to access payload on a FAIL result.");
    }

    return std::make_unique<SharedData>(inner->payload);
}

const std::string &extract_error_result(const std::unique_ptr<OpaqueResult> &r)
{
    Result *inner = reinterpret_cast<Result *>(r->ptr);
    if (inner->type != Result::FAIL)
    {
        throw std::runtime_error("Attempted to access failure reason on a OK result.");
    }

    return inner->fail_rsn;
}

int get_result_status(const std::unique_ptr<OpaqueResult> &r) noexcept
{
    Result *inner = reinterpret_cast<Result *>(r->ptr);
    return static_cast<int>(inner->type);
}
