#pragma once

#include <string>
#include <optional>
#include "string_utils.hpp"

class ClassificationResult
{
public:
    static ClassificationResult ok() { return {}; }
    static ClassificationResult fail(std::string reason) { return ClassificationResult{std::move(reason)}; }

    bool is_ok() const noexcept { return !reason.has_value(); }
    const std::string &failure() const { return *reason; }

    ClassificationResult() = default;
    ~ClassificationResult() = default;

    explicit ClassificationResult(std::string r) : reason(std::move(r)) {}

private:
    std::optional<std::string> reason;
};

class ExtractionResult
{
public:
    static inline ExtractionResult ok(IntoString auto &&data)
    {
        ExtractionResult r;
        r.extracted_data = as_string(std::forward<decltype(data)>(data));
        return r;
    }

    static inline ExtractionResult fail(std::string reason) { return ExtractionResult{std::move(reason)}; }

    bool is_ok() const noexcept { return !fail_reason.has_value(); }
    const std::string &failure() const { return *fail_reason; }
    const std::string &data() const { return *extracted_data; }
    std::string take_data() && { return std::move(*extracted_data); }

    ExtractionResult() = default;
    ~ExtractionResult() = default;

    explicit ExtractionResult(std::string r) : fail_reason(std::move(r)) {}
private:
    std::optional<std::string> extracted_data;
    std::optional<std::string> fail_reason;
};
