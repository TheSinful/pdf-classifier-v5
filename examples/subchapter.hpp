#pragma once

#include <pdf_classifier_lib/object.hpp>
#include <pdf_classifier_lib/result.hpp>
#include <shared/result.h>
#include <string>

class SubChapter : public Object {
public:
  SubChapter(int page_num) : Object(page_num) {}

  ClassificationResult contains_valid_subchapter_text();
  ClassificationResult extract_subchapter_num();

  std::string subchapter_number;
};

inline constexpr int UPPERCASE_SUBCHAPTER_TEXT_FREQUENCY = 1;
inline constexpr int LOWERCASE_SUBCHAPTER_TEXT_FREQUENCY = 1;

