#include "subchapter.hpp"
#include "result.hpp"
#include <format>
#include <regex>

ClassificationResult SubChapter::contains_valid_subchapter_text() {

  int uppercase_frequency = frequency_of("CHAPTER", compressed_text, 1);
  int lowercase_frequency = frequency_of("Chapter", compressed_text, 1);

  if (uppercase_frequency != UPPERCASE_SUBCHAPTER_TEXT_FREQUENCY)
    return ClassificationResult::fail(std::format("expected {} instances of the string 'CHAPTER' but found {} instances.",
                                                  UPPERCASE_SUBCHAPTER_TEXT_FREQUENCY, uppercase_frequency));

  if (lowercase_frequency != LOWERCASE_SUBCHAPTER_TEXT_FREQUENCY)
    return ClassificationResult::fail(std::format("expected {} instances of the string 'Chapter' but found {} instances.",
                                                  LOWERCASE_SUBCHAPTER_TEXT_FREQUENCY, lowercase_frequency));

  for (PdfText& entry : extracted_text) {
    if (!contains_text("CHAPTER", entry.text)) {
      continue;
    }

    bool meets_lower_fontsize_bound = std::abs(entry.font_size - SUBCHAPTER_SIZE_LOWER_BOUND) <= SUBCHAPTER_SIZE_THRESHOLD;
    bool meets_upper_fontsize_bound = std::abs(entry.font_size - SUBCHAPTER_SIZE_UPPER_BOUND) <= SUBCHAPTER_SIZE_THRESHOLD;

    if (!meets_lower_fontsize_bound && !meets_upper_fontsize_bound) {
      continue;
    }

    return ClassificationResult::ok();
  }

  return ClassificationResult::fail("no text entry fit expected criteria");
}

ClassificationResult SubChapter::extract_subchapter_num() {
  std::smatch match;

  if (std::regex_search(compressed_text, match, SUBCHAPTER_NUM_PATTERN)) {
    subchapter_number = match[1].str();
    return ClassificationResult::ok();
  }

  return ClassificationResult::fail("no chapter number could be found within text of page.");
}

ClassificationResult SubChapter::classify(Attached&) {
  UNWRAP(contains_valid_subchapter_text());
  UNWRAP(extract_subchapter_num());

  return ClassificationResult::ok();
}

ExtractionResult SubChapter::extract(Attached&) {
  return ExtractionResult::ok(nlohmann::json{{"subchapter_num", subchapter_number}});
}


