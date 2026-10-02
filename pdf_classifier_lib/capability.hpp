#pragma once

#include "attached.hpp"
#include <string>
#include <vector>

struct CapabilityFailure {
  CapabilityFailure(const std::string_view capability, const std::string reason)
      : capability(capability), reason(reason) {}

  const std::string_view capability; // should not ever be uninitialized
  const std::string reason{"unknown"};
};

class Capability {
protected:
  virtual const std::string_view name() = 0;

  virtual std::vector<CapabilityFailure>
  evaluate_capability(Attached &att) noexcept = 0;

  CapabilityFailure construct_failure(const std::string reason) {
    return CapabilityFailure(this->name(), reason);
  }
};

class TextExtraction : public Capability {
public:
  const std::string_view name() override { return "TextExtraction"; }

  std::vector<CapabilityFailure>
  evaluate_capability(Attached &att) noexcept override {

    std::vector<CapabilityFailure> failures = {};

    try {
      this->extracted_text = att.extract_text();
    } catch (FzError err) {
      failures.push_back(construct_failure(err.what()));
      return failures;
    }

    this->compressed_text = compress_text(this->extracted_text);

    return failures;
  }

protected:
  std::vector<PdfText> extracted_text = {};
  std::string compressed_text{""}; // extracted_text compressed into one string
};

