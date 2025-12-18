#pragma once

#define NDEBUG
#ifdef NDEBUG

#include <exception>
#include <format>
#include <string>

class AssertionError : public std::exception
{
public:
    std::string msg;

    AssertionError(std::string msg) : msg(msg) {};

    const char *what() const override
    {
        return std::format("Assertion failed: {}", msg).c_str();
    }
};

/// @brief Asserts that the condition provided is true
/// If it isn't true, then throws an AssertionError
#define DEBUG_ASSERT(condition, string) \
    if (condition)                      \
    {                                   \
    }                                   \
    else                                \
    {                                   \
        throw AssertionError(string);   \
    }

#else

#define ASSERT(condition, string)

#endif