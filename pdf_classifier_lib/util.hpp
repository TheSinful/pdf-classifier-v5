#pragma once

#define UNWRAP(expr)               \
    do                             \
    {                              \
        auto _unwrap_res = (expr); \
        if (!_unwrap_res.is_ok())  \
            return _unwrap_res;    \
    } while (0)