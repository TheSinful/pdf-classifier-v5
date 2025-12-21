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

std::unique_ptr<SharedData> call_classify(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc, const std::string &obj)
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
        typedef void *(*classify_func)(fz_context *, fz_document *);
        classify_func fn = reinterpret_cast<classify_func>(ptr);
        void *res = fn(ctx, doc);
        if (!res)
        {
            throw std::runtime_error("classify returned null for " + obj);
        }
        return std::make_unique<SharedData>(res);
    }
    else
    {
        throw std::runtime_error("couldn't find classify func ptr " + obj);
    }
}

void call_extract(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc, const std::unique_ptr<SharedData> &shared, const std::string &obj)
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
        typedef void (*classify_func)(fz_context *, fz_document *, void *);
        classify_func fn = reinterpret_cast<classify_func>(ptr);
        return fn(ctx, doc, shared->ptr);
    }
    else
    {
        throw std::runtime_error("couldn't find classify func ptr " + obj);
    }
}
