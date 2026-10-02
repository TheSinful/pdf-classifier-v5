#include "chapter.hpp"
#include <format>
#include <regex>
#include <pdf_classifier_lib/string_utils.hpp>

using nlohmann::json;

ClassificationResult Chapter::classify(Attached& att) {
  this->extracted_text = att.extract_text();
  this->compressed_text = compress_text(this->extracted_text);


  UNWRAP(this->contains_valid_chapter_text());
  UNWRAP(this->extract_chapter_number());

  return ClassificationResult::ok();
}

ExtractionResult Chapter::extract(Attached& att) { return this->extract_expected_subchapters(); }

ClassificationResult Chapter::contains_valid_chapter_text() {
  int upper_freq = frequency_of("CHAPTER", compressed_text, 1);
  int lower_freq = frequency_of("Chapter", compressed_text, 1);

  if (upper_freq != EXPECTED_UPPERCASE_CHAPTER_FREQUENCY) {
    return ClassificationResult::fail(
        std::format("found {} instances of 'CHAPTER'(uppercase), but expected {}", upper_freq, EXPECTED_UPPERCASE_CHAPTER_FREQUENCY));
  }

  if (lower_freq != EXPECTED_LOWERCASE_CHAPTER_FREQUENCY) {
    return ClassificationResult::fail(
        std::format("found {} instances of 'chapter'(lowercase), but expected {}", lower_freq, EXPECTED_LOWERCASE_CHAPTER_FREQUENCY));
  }

  for (PdfText& entry : extracted_text) {
    if (!contains_text("CHAPTER", entry.text))
      continue;

    if (!contains_text("Helvetica", entry.font_name))
      continue;

    // 10.4..11.0 font size range
    bool is_in_lower_bound = std::abs(entry.font_size - CHAPTER_FONT_SIZE_LOWER_BOUND) <= CHAPTER_FONT_SIZE_TOLERANCE;
    bool is_in_upper_bound = std::abs(entry.font_size - CHAPTER_FONT_SIZE_UPPER_BOUND) <= CHAPTER_FONT_SIZE_TOLERANCE;
    if (!is_in_lower_bound && !is_in_upper_bound)
      continue;

    return ClassificationResult::ok();
  }

  return ClassificationResult::fail("extracted text didn't have any entries with expected structure.");
}

ClassificationResult Chapter::extract_chapter_number() {
  std::smatch match;

  if (std::regex_search(compressed_text, match, CHAPTER_NUM_PATTERN)) {
    chapter_number = match[1].str();
    return ClassificationResult::ok();
  }

  return ClassificationResult::fail("no chapter number could be found within text of page.");
}

ExtractionResult Chapter::extract_expected_subchapters() {
  std::regex subchapter_num_pattern = std::regex(std::format(R"({}-\d+)", chapter_number), std::regex_constants::icase);
  std::vector<ExpectedSubchapter> expected_subchapters = {};

  for (std::sregex_iterator it(compressed_text.begin(), compressed_text.end(), subchapter_num_pattern);
       it != std::sregex_iterator{}; ++it) {
    const std::smatch& match = *it;
    std::string matched_text = match.str();

    ExpectedSubchapter subchapter;
    subchapter.sub_chapter_num = matched_text;
    subchapter.path = std::format("./sub_chapter_{}", matched_text);

    expected_subchapters.emplace_back(subchapter);
  }

  json data = nlohmann::json{{"chapter_num", this->chapter_number}, {"sub_chapters", expected_subchapters}};
  return ExtractionResult::ok(data);
}

DEFINE_OBJECT(chapter, Chapter);

