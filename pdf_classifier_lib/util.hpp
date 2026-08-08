#pragma once

#define UNWRAP(expr)               \
    do                             \
    {                              \
        auto _unwrap_res = (expr); \
        if (!_unwrap_res.is_ok())  \
            return _unwrap_res;    \
    } while (0)

#define UNWRAP_RAW_RESULT(res)                 \
    do                                         \
    {                                          \
        Result *__res = res;                   \
        if (__res->type == Result::Type::FAIL) \
        {                                      \
            return __res;                      \
        }                                      \
        else                                   \
        {                                      \
            delete __res;                      \
        }                                      \
    } while (0)

#define ASSERT_RAW_RESULT(condition, fmt, ...)                  \
    do                                                          \
    {                                                           \
        if (condition)                                          \
        {                                                       \
            return Result::fail(std::format(fmt, __VA_ARGS__)); \
        }                                                       \
    } while (0)