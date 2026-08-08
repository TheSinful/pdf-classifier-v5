#pragma once

#include <shared/result.h>
#include <vector>
#include <memory>
#include "result.hpp"
#include "wrappers.hpp"
#include "util.hpp"
#include "string_utils.hpp"

class AttachedAllocator
{
private:
    fz_context *ctx = nullptr;
    std::vector<std::unique_ptr<FzOwnedResource>> resources;

public:
    template <class Owned, class New, class... Args>
    typename Owned::element_type *
    allocate(New new_fn, Args &&...args)
    {
        auto owned = std::make_unique<Owned>(
            Owned::make(ctx, new_fn, std::forward<Args>(args)...));

        auto *raw = owned->get();
        resources.push_back(std::move(owned));
        return raw;
    }

    AttachedAllocator(fz_context *ctx) : ctx(ctx) {}
};

class Attached
{
private:
    fz_context *_ctx = nullptr;
    fz_document *_doc = nullptr;
    FzPage _page;
    uint32_t _page_num = 0;
    AttachedAllocator alloc;

public:
    explicit Attached(fz_context *ctx, fz_document *doc, uint32_t page) : _ctx(ctx), _doc(doc), _page_num(page),
                                                                          _page(FzPage::make(ctx, fz_load_page, _doc, page)),
                                                                          alloc(AttachedAllocator(ctx)) {}

    fz_context *raw_ctx() const { return _ctx; }
    fz_document *raw_doc() const { return _doc; }
    fz_page *raw_page() const { return _page.get(); }
    uint32_t page_num() const { return _page_num; }

    bool has_image() const { return ::has_image(raw_ctx(), raw_page()); }
    std::vector<PdfText> extract_text() const { return ::extract_text(raw_ctx(), raw_page(), page_num()); }

    /// Create a MuPDF resource owned by this call. The context is injected, the
    /// call is guarded, and the resource is dropped when this Attached dies.
    template <class Owned, class New, class... Args>
    typename Owned::element_type *allocate(New new_fn, Args &&...args)
    {
        return alloc.allocate<Owned>(new_fn, std::forward<Args>(args)...);
    }
};

class Object
{
public:
    virtual ClassificationResult classify(Attached &) = 0;
    virtual ExtractionResult extract(Attached &) = 0;
    virtual ~Object() = default;
    explicit Object(int page_num) : page_num_(page_num) {}

protected:
    int page_num() { return page_num_; }

private:
    int page_num_ = 0;
};

#define DEFINE_OBJECT(name, Type)                                                                         \
                                                                                                          \
    void deleter_##Type(void *p) noexcept                                                                 \
    {                                                                                                     \
        delete static_cast<Type *>(p);                                                                    \
    }                                                                                                     \
                                                                                                          \
    Result *classify_##name(uint32_t page_num, fz_context *ctx,                                           \
                            fz_document *doc)                                                             \
    {                                                                                                     \
                                                                                                          \
        static_assert(std::is_base_of_v<Object, Type>, "...must derive from Object");                     \
        static_assert(std::is_constructible_v<Type, uint32_t>, "...needs a (uint32_t page) constructor"); \
        static_assert(!std::is_abstract_v<Type>, "...must implement both classify() and extract()");      \
                                                                                                          \
        auto obj = std::make_unique<Type>(page_num);                                                      \
        {                                                                                                 \
            Attached att(ctx, doc, page_num);                                                             \
            ClassificationResult out = obj->classify(att);                                                \
            if (!out.is_ok())                                                                             \
                return Result::fail(out.failure());                                                       \
        }                                                                                                 \
        return Result::ok(obj.release(), &deleter_##Type);                                                \
    }                                                                                                     \
                                                                                                          \
    Result *extract_##name(uint32_t page_num, fz_context *ctx,                                            \
                           fz_document *doc, void *shared)                                                \
    {                                                                                                     \
        Type *obj = static_cast<Type *>(shared);                                                          \
        Attached att(ctx, doc, page_num);                                                                 \
        ExtractionResult out = obj->extract(att);                                                         \
        if (!out.is_ok())                                                                                 \
            return Result::fail(out.failure());                                                           \
        return Result::ok(new std::string(std::move(out).take_data()), &deleter_StdString);               \
    }                                                                                                     \
    static_assert(true, "")
