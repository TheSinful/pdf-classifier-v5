#pragma once

#include <mupdf/fitz.h>
#include <nlohmann/json.hpp>
#include <pdf_classifier_lib/capability.hpp>
#include <pdf_classifier_lib/object.hpp>
#include <pdf_classifier_lib/result.hpp>
#include <regex>
#include <string>

inline constexpr int EXPECTED_UPPERCASE_CHAPTER_FREQUENCY = 1;
inline constexpr int EXPECTED_LOWERCASE_CHAPTER_FREQUENCY = 2;
inline constexpr float CHAPTER_FONT_SIZE_LOWER_BOUND = 10.4;
inline constexpr float CHAPTER_FONT_SIZE_UPPER_BOUND = 11.0;
inline constexpr float CHAPTER_FONT_SIZE_TOLERANCE = 0.3;
// CHAPTER followed by X-Y format
inline const std::regex CHAPTER_NUM_PATTERN(R"(CHAPTER\s+(\d+-\d+))", std::regex_constants::icase);

struct ExpectedSubchapter {
  std::string sub_chapter_num;
  std::string path;
};

NLOHMANN_DEFINE_TYPE_NON_INTRUSIVE(ExpectedSubchapter, sub_chapter_num, path)

class Chapter : public ObjectWith<TextExtraction> {
public:
  explicit Chapter(int page_num, Attached& att) : ObjectWith(page_num, att) {}

  ClassificationResult classify(Attached& att) override;
  ExtractionResult extract(Attached& att) override;

  ClassificationResult evaluate_capability_failures(const std::vector<CapabilityFailure> &failures) override {
    return ClassificationResult::fail(failures.front().reason);  // no text: can't be a chapter
  }

private:
  std::string chapter_number{"uninitialized"};

  ClassificationResult contains_valid_chapter_text();
  ClassificationResult extract_chapter_number();
  ExtractionResult extract_expected_subchapters();
};
