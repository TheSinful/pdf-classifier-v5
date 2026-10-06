#pragma once

#include <pdf_classifier_lib/object.hpp>
#include <pdf_classifier_lib/result.hpp>
#include <regex>
#include <shared/result.h>
#include <string>


// "CHAPTER x-y-z" where x, y, and z are numbers.
inline const std::regex SUBCHAPTER_NUM_PATTERN(R"(CHAPTER\s+(\d+-\d+-\d+))", std::regex_constants::icase);
inline constexpr int UPPERCASE_SUBCHAPTER_TEXT_FREQUENCY = 1;
inline constexpr int LOWERCASE_SUBCHAPTER_TEXT_FREQUENCY = 1;
inline constexpr float SUBCHAPTER_SIZE_LOWER_BOUND = 10.4; 
inline constexpr float SUBCHAPTER_SIZE_UPPER_BOUND = 11.0; 
inline constexpr float SUBCHAPTER_SIZE_THRESHOLD = 2.3;  

class SubChapter : public ObjectWith<TextExtraction> {
public:
  explicit SubChapter(int page_num, Attached& att) : ObjectWith(page_num, att) {}

  ClassificationResult classify(Attached&) override;
  ExtractionResult extract(Attached&) override;

  ClassificationResult evaluate_capability_failures(const std::vector<CapabilityFailure>& failures) override {
    return ClassificationResult::fail(failures.front().reason);
  }

  ClassificationResult contains_valid_subchapter_text();
  ClassificationResult extract_subchapter_num();

  std::string subchapter_number{"uninitialized"};
};

DEFINE_OBJECT(subchapter, SubChapter);
