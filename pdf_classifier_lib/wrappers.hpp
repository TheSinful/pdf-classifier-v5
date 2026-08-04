#pragma once

#include <mupdf/fitz.h>
#include <utility>
#include <stdexcept>
#include <type_traits>

class FzError : public std::runtime_error
{
public:
    FzError(int code, const char *msg) : std::runtime_error(msg), code(code) {}

    int code; // fz_caught(): FZ_ERROR_MEMORY, FZ_ERROR_SYNTAX, ...
};

template <class Fn, class... Args>
auto fz_call(fz_context *ctx, Fn fn, Args &&...args)
    -> std::invoke_result_t<Fn, fz_context *, Args &&...>
{
    using R = std::invoke_result_t<Fn, fz_context *, Args &&...>;

    if constexpr (std::is_void_v<R>)
    {
        fz_try(ctx)
        {
            fn(ctx, std::forward<Args>(args)...);
        }
        fz_catch(ctx)
        {
            throw FzError(fz_caught(ctx), fz_caught_message(ctx));
        }
    }
    else
    {
        R result{};
        fz_var(result);

        fz_try(ctx)
        {
            result = fn(ctx, std::forward<Args>(args)...);
        }
        fz_catch(ctx)
        {
            throw FzError(fz_caught(ctx), fz_caught_message(ctx));
        }

        return result;
    }
}

struct FzOwnedResource
{
    virtual ~FzOwnedResource() = default;
};

template <class T, void (*Drop)(fz_context *, T *)>
class FzOwned : public FzOwnedResource
{
public:
    using element_type = T;

    FzOwned() noexcept = default;
    FzOwned(fz_context *ctx, T *owned) noexcept : ctx(ctx), owned(owned) {}
    ~FzOwned() { reset(); }

    FzOwned(const FzOwned &) = delete;
    FzOwned &operator=(const FzOwned &) = delete;

    FzOwned(FzOwned &&o) noexcept
        : ctx(std::exchange(o.ctx, nullptr)), owned(std::exchange(o.owned, nullptr)) {}

    FzOwned &operator=(FzOwned &&rhs) noexcept
    {
        if (this != &rhs)
        {
            reset();
            ctx = std::exchange(rhs.ctx, nullptr);
            owned = std::exchange(rhs.owned, nullptr);
        }
        return *this;
    }

    template <class New, class... Args>
    static FzOwned make(fz_context *ctx, New new_fn, Args &&...args)
    {
        static_assert(
            std::is_invocable_v<New, fz_context *, Args &&...>,
            "constructor is not callable with these arguments - note that ctx is "
            "injected automatically, so don't pass it yourself");

        static_assert(
            std::is_same_v<std::invoke_result_t<New, fz_context *, Args &&...>, T *>,
            "constructor must return T* for this FzOwned specialisation");

        return FzOwned{ctx, fz_call(ctx, new_fn, std::forward<Args>(args)...)};
    }

    T *get() const noexcept { return owned; }

    explicit operator bool() const noexcept { return owned != nullptr; }

    [[nodiscard]]
    T *release() noexcept
    {
        ctx = nullptr;
        return std::exchange(owned, nullptr);
    }

    void reset() noexcept
    {
        if (owned)
            Drop(ctx, owned);
        owned = nullptr;
        ctx = nullptr;
    }

private:
    fz_context *ctx = nullptr;
    T *owned = nullptr;
};

using FzPage = FzOwned<fz_page, fz_drop_page>;
using FzSTextPage = FzOwned<fz_stext_page, fz_drop_stext_page>;
using FzPixmap = FzOwned<fz_pixmap, fz_drop_pixmap>;
