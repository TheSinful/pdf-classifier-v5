#pragma once

#define PDF_ASSERT(condition, fmt, ...)                                                                                \
  do {                                                                                                                 \
    if (condition) {                                                                                                   \
      return Result::fail(std::format(fmt, __VA_ARGS__));                                                              \
    }                                                                                                                  \
  } while (0)

#define UNWRAP_RESULT(res)                                                                                             \
  do {                                                                                                                 \
    Result* __res = res;                                                                                               \
    if (__res->type == Result::Type::FAIL) {                                                                           \
      return __res;                                                                                                    \
    } else {                                                                                                           \
      delete __res;                                                                                                    \
    }                                                                                                                  \
  } while (0)
