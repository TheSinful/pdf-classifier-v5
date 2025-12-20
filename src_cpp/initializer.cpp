#include "initializer.h"
#include <shared/generated_page_types.h>
#include <shared/func_map.h>

std::unique_ptr<OpaqueCtx> create_new_ctx(size_t mem_limit)
{
    return std::make_unique<OpaqueCtx>(fz_new_context(NULL, NULL, mem_limit));
}

std::unique_ptr<OpaqueDoc> create_new_doc(const std::string &doc_path, const OpaqueCtx &o_ctx)
{
    fz_context *ctx = static_cast<fz_context *>(o_ctx.ptr);

    return std::make_unique<OpaqueDoc>(fz_open_document(ctx, doc_path.c_str()));
}

std::unique_ptr<SharedData> call_classify(const OpaqueCtx &o_ctx, const OpaqueDoc &o_doc, const std::string &obj)
{
    fz_context *ctx = static_cast<fz_context *>(o_ctx.ptr);
    fz_document *doc = static_cast<fz_document *>(o_doc.ptr);

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

    func_ptr ptr = found_func->ptr;
    if (ptr)
    {
        typedef SharedData (*classify_func)(fz_context *, fz_document *);
        classify_func fn = reinterpret_cast<classify_func>(ptr);
        return std::make_unique<SharedData>(&fn(ctx, doc));
    }
    else
    {
        throw std::runtime_error("couldn't find classify func ptr " + obj);
    }
}

void call_extract(const OpaqueCtx &o_ctx, const OpaqueDoc &o_doc, const SharedData &shared, const std::string &obj)
{
    fz_context *ctx = static_cast<fz_context *>(o_ctx.ptr);
    fz_document *doc = static_cast<fz_document *>(o_doc.ptr);

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

    func_ptr ptr = found_func->ptr;
    if (ptr)
    {
        typedef void (*classify_func)(fz_context *, fz_document *, void *);
        classify_func fn = reinterpret_cast<classify_func>(ptr);
        return fn(ctx, doc, shared.ptr);
    }
    else
    {
        throw std::runtime_error("couldn't find classify func ptr " + obj);
    }
}
