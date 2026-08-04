#include "ffi.hpp"
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

class ClonedCtx
{
public:
    explicit ClonedCtx(fz_context *master)
        : clone(fz_clone_context(master))
    {
        if (!clone)
            throw std::runtime_error(
                "fz_clone_context returned null - the master context was created "
                "with default locks; install lock functions in create_new_ctx");
    }

    ~ClonedCtx()
    {
        if (clone)
            fz_drop_context(clone);
    }

    ClonedCtx(const ClonedCtx &) = delete;
    ClonedCtx &operator=(const ClonedCtx &) = delete;
    ClonedCtx(ClonedCtx &&) = delete;
    ClonedCtx &operator=(ClonedCtx &&) = delete;

    fz_context *get() const noexcept { return clone; }

private:
    fz_context *clone = nullptr;
};

static void no_op_lock_fn(void *user, int lock) {}
static void no_op_unlock_fn(void *user, int lock) {}

fz_locks_context *no_op_locks()
{
    fz_locks_context no_op_locks = {nullptr, no_op_lock_fn, no_op_unlock_fn};

    return &no_op_locks;
}

std::unique_ptr<OpaqueCtx> create_new_ctx(size_t mem_limit)
{
    fz_context *ctx = fz_new_context(NULL, no_op_locks(), mem_limit);
    // we take a no-op locks to clone the context before passing to the user defined classify/extraction function
    // which ensures that the user's manipulation of ctx is 'sandboxed' in such a manner that another
    // call on the same thread does not poison the master ctx. the master ctx is the ctx owned by core (Rust)
    // which is what we construct here before passing ownership

    if (!ctx)
        throw std::runtime_error("Failed to create context!");

    return std::make_unique<OpaqueCtx>(ctx);
}

std::unique_ptr<OpaqueDoc> create_new_doc(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::string &doc_path)
{
    if (!std::filesystem::exists(doc_path))
        throw std::runtime_error("File doesn't exist at path provided " + doc_path);

    fz_context *ctx = cast_opaque_ctx(o_ctx);

    if (!ctx)
        throw std::runtime_error("Failed to access created context!");

    fz_try(ctx)
    {
        fz_register_document_handlers(ctx);
    }
    fz_catch(ctx)
    {
        THROW_MUPDF_ERROR("Failed to register document handlers!");
    }

    fz_document *doc = nullptr;
    fz_try(ctx)
    {
        doc = fz_open_document(ctx, doc_path.c_str());
    }
    fz_catch(ctx)
    {
        THROW_MUPDF_ERROR("Failed to create document!");
    }

    return std::make_unique<OpaqueDoc>(doc);
}

std::unique_ptr<OpaqueResult> call_classify(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc, const std::string &obj, uint32_t page)
{
    ClonedCtx ctx = ClonedCtx(cast_opaque_ctx(o_ctx));
    fz_document *doc = cast_opaque_doc(o_doc);

    const Func *found_func = nullptr;
    for (const auto &func : ClassifyFuncMap)
    {
        if (func.obj_name == obj)
        {
            found_func = &func;
            break;
        }
    }

    if (!found_func)
        throw std::runtime_error("couldn't find obj: '" + obj + "' in generated func map!");

    void *ptr = found_func->ptr;
    if (!ptr)
        throw std::runtime_error("couldn't call classify func ptr " + obj);

    typedef Result *(*classify_func)(uint32_t, fz_context *, fz_document *);
    classify_func fn = reinterpret_cast<classify_func>(ptr);

    Result *res = nullptr;
    try
    {
        res = fn(page, ctx.get(), doc);
    }
    catch (const std::exception &e)
    {
        return std::make_unique<OpaqueResult>(Result::fail(e.what()));
    }

    if (!res)
        throw std::runtime_error("classify returned nullptr for " + obj);

    return std::make_unique<OpaqueResult>(res);
}

std::unique_ptr<OpaqueResult> call_extract(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc, const std::unique_ptr<SharedData> &shared, const std::string &obj, uint32_t page)
{
    ClonedCtx ctx = ClonedCtx(cast_opaque_ctx(o_ctx));
    fz_document *doc = cast_opaque_doc(o_doc);

    const Func *found_func = nullptr;
    for (const auto &func : ExtractFuncMap)
    {
        if (func.obj_name == obj)
        {
            found_func = &func;
            break;
        }
    }

    if (found_func == nullptr)
        throw std::runtime_error("couldn't find obj: " + obj + "in generated func map!");

    void *ptr = found_func->ptr;
    if (!ptr)
        throw std::runtime_error("couldn't find extract func ptr " + obj);

    typedef Result *(*extract_func)(uint32_t, fz_context *, fz_document *, void *);
    extract_func fn = reinterpret_cast<extract_func>(ptr);

    Result *res = nullptr;
    try
    {
        res = fn(page, ctx.get(), doc, shared->ptr);
    }
    catch (const std::exception &e)
    {
        return std::make_unique<OpaqueResult>(Result::fail(e.what()));
    }

    if (!res)
        throw std::runtime_error("extract returned nullptr for " + obj);

    return std::make_unique<OpaqueResult>(res);
}

inline Result *cast_opaque_result(const std::unique_ptr<OpaqueResult> &o_res)
{
    if (!o_res)
        throw std::runtime_error("Attempted to cast nullptr as a result!");

    return static_cast<Result *>(o_res->ptr);
}

inline fz_context *cast_opaque_ctx(const std::unique_ptr<OpaqueCtx> &o_ctx)
{
    if (!o_ctx)
        throw std::runtime_error("Attempted to cast nullptr as context!");

    return static_cast<fz_context *>(o_ctx->ptr);
}

inline fz_document *cast_opaque_doc(const std::unique_ptr<OpaqueDoc> &o_doc)
{
    if (!o_doc)
        throw std::runtime_error("Attepted to cast nullptr as document!");

    return static_cast<fz_document *>(o_doc->ptr);
}

void drop_ctx(const std::unique_ptr<OpaqueCtx> &o_ctx)
{
    fz_drop_context(cast_opaque_ctx(o_ctx));
}

void drop_doc(const std::unique_ptr<OpaqueCtx> &o_ctx, const std::unique_ptr<OpaqueDoc> &o_doc)
{
    fz_drop_document(cast_opaque_ctx(o_ctx), cast_opaque_doc(o_doc));
}

void drop_result(const std::unique_ptr<OpaqueResult> &r)
{
    if (!r)
        throw std::runtime_error("Attempted to drop a result that was already dropped!");

    Result *inner = cast_opaque_result(r);

    if (inner->type == Result::Type::OK && inner->deleter)
        inner->deleter(inner->payload);

    // inner->fail_rsn is implicitly destructed via delete
    delete inner;
}

std::unique_ptr<SharedData> extract_shared_payload(const std::unique_ptr<OpaqueResult> &r)
{
    if (!r)
        throw std::runtime_error("Attempted to access payload on a nullptr");

    Result *inner = cast_opaque_result(r);

    if (inner->type != Result::OK)
        throw std::runtime_error("Attempted to access payload on a FAIL result.");

    std::unique_ptr<SharedData> ptr = std::make_unique<SharedData>(inner->payload, inner->deleter);
    inner->payload = nullptr;
    inner->deleter = nullptr;

    return ptr;
}

const std::string &extract_error_result(const std::unique_ptr<OpaqueResult> &r)
{
    if (!r)
        throw std::runtime_error("Attempted to access failure reason on a nullptr!");

    Result *inner = cast_opaque_result(r);

    if (inner->type != Result::FAIL)
        throw std::runtime_error("Attempted to access failure reason on a OK result.");

    return inner->fail_rsn;
}

const std::string &extract_string_payload(const std::unique_ptr<OpaqueResult> &r)
{
    if (!r)
        throw std::runtime_error("Attempted to access string payload on a nullptr!");

    Result *inner = cast_opaque_result(r);

    if (inner->type == Result::FAIL)
        throw std::runtime_error("Attempted to access string payload on a FAIL result.");

    if (inner->payload == NULL)
        throw std::runtime_error("Given nullptr for string payload!");

    return *static_cast<std::string *>(inner->payload);
}

int get_result_status(const std::unique_ptr<OpaqueResult> &r)
{
    if (!r)
        throw std::runtime_error("Attempted to get result status of a null result!");

    Result *inner = cast_opaque_result(r);

    return static_cast<int>(inner->type);
}
