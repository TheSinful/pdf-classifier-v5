#pragma once

#include "generated/fz_bindings.hpp"
#include <mupdf/fitz.h>
#include <stdexcept>
#include <type_traits>
#include <utility>

inline thread_local fz_context *current_context = nullptr;
inline void bind_current_context(fz_context *to) {
    current_context = to;
}

class FzError : public std::runtime_error {
  public:
    FzError(int code, const char *msg) : std::runtime_error(msg), code(code) {}

    int code; // fz_caught(): FZ_ERROR_MEMORY, FZ_ERROR_SYNTAX, ...
};

template <class Fn, class... Args>
auto fz_call(fz_context *ctx, Fn fn, Args &&...args)
    -> std::invoke_result_t<Fn, fz_context *, Args &&...> {
    using R = std::invoke_result_t<Fn, fz_context *, Args &&...>;

    if constexpr (std::is_void_v<R>) {
        fz_try(ctx) {
            fn(ctx, std::forward<Args>(args)...);
        }
        fz_catch(ctx) {
            throw FzError(fz_caught(ctx), fz_caught_message(ctx));
        }
    } else {
        R result{};
        fz_var(result);

        fz_try(ctx) {
            result = fn(ctx, std::forward<Args>(args)...);
        }
        fz_catch(ctx) {
            throw FzError(fz_caught(ctx), fz_caught_message(ctx));
        }

        return result;
    }
}

struct FzOwnedResource {
    virtual ~FzOwnedResource() = default;
};

template <class T, void (*Drop)(fz_context *, T *), FzBinding bound_to>
class FzOwned : public FzOwnedResource {
  public:
    using element_type = T;

    FzOwned() noexcept = default;
    FzOwned(T *owned) noexcept : owned(owned) {}
    ~FzOwned() {
        reset();
    }

    FzOwned(const FzOwned &) = delete;
    FzOwned &operator=(const FzOwned &) = delete;

    FzOwned(FzOwned &&o) noexcept : owned(std::exchange(o.owned, nullptr)) {}

    FzOwned &operator=(FzOwned &&rhs) noexcept {
        if (this != &rhs) {
            reset();
            owned = std::exchange(rhs.owned, nullptr);
        }
        return *this;
    }

    // The passed in ctx is a clone of [current_context] created on the rust end when either classify or extract are called.
    template <class New, class... Args>
    static FzOwned make(fz_context *ctx, New new_fn, Args &&...args) {
        static_assert(
            std::is_invocable_v<New, fz_context *, Args &&...>,
            "constructor is not callable with these arguments - note that ctx is "
            "injected automatically, so don't pass it yourself");

        static_assert(
            std::is_same_v<std::invoke_result_t<New, fz_context *, Args &&...>,
                           T *>,
            "constructor must return T* for this FzOwned specialisation");

        return FzOwned{fz_call(ctx, new_fn, std::forward<Args>(args)...)};
    }

    T *get() const noexcept {
        return owned;
    }

    explicit operator bool() const noexcept {
        return owned != nullptr;
    }

    [[nodiscard]]
    T *release() noexcept {
        return std::exchange(owned, nullptr);
    }

    void reset() {
        if (owned) {
            if (!current_context) {
                throw std::runtime_error("No bound context!");
            }

            Drop(current_context, owned);
        }
        owned = nullptr;
    }

  private:
    T *owned = nullptr;
};

template <class T>
class KeptResource {
    static_assert(fz_binding_v<typename T::element_type> == FzBinding::Context, "KeptResource<T> only holds context-bound MuPDF objects, not document-bound objects.");

  public:
    explicit KeptResource(T owned) : owned(std::move(owned)) {}

  private:
    T owned;
};
