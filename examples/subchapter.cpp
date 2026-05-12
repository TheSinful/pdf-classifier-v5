#include "subchapter.hpp"
#include "util.hpp"
#include <memory>
#include <regex>

Result* SubChapter::contains_valid_subchapter_text() {

  int uppercase_frequency = frequency_of("CHAPTER", compressed_text, 1);
  int lowercase_frequency = frequency_of("Chapter", compressed_text, 1);

  PDF_ASSERT(uppercase_frequency != UPPERCASE_SUBCHAPTER_TEXT_FREQUENCY,
             "expected {} instances of the string 'CHAPTER' but found {} instances.", UPPERCASE_SUBCHAPTER_TEXT_FREQUENCY,
             uppercase_frequency);

  PDF_ASSERT(lowercase_frequency != LOWERCASE_SUBCHAPTER_TEXT_FREQUENCY,
             "expected {} instances of the string 'Chapter' but found {} instances.", LOWERCASE_SUBCHAPTER_TEXT_FREQUENCY,
             lowercase_frequency);

  for (PdfText& entry : extracted_text) {
    if (!contains_text("CHAPTER", entry.text)) {
      continue;
    }

    bool meets_lower_fontsize_bound = std::abs(entry.font_size - 10.4) <= 2.3;
    bool meets_upper_fontsize_bound = std::abs(entry.font_size - 11.0) <= 2.3;

    if (!meets_lower_fontsize_bound && !meets_upper_fontsize_bound) {
      continue;
    }

    return Result::ok(NULL, NULL);
  }

  return Result::fail("no text entry fit expected criteria");
}

Result* SubChapter::extract_subchapter_num() {
  // "CHAPTER x-y-z" where x, y, and z are numbers.
  std::regex pattern = std::regex(R"(CHAPTER\s+(\d+-\d+-\d+))", std::regex_constants::icase);
  std::smatch match;

  if (std::regex_search(compressed_text, match, pattern)) {
    subchapter_number = match[1].str();
    return Result::ok(NULL, NULL);
  }

  return Result::fail("no chapter number could be found within text of page.");
}

void deleter_SubChapter(void* p) { delete static_cast<SubChapter*>(p); }

Result* classify_subchapter(uint32_t page, fz_context* ctx, fz_document* doc) {
  auto inst = std::make_unique<SubChapter>(ctx, doc, page);

  UNWRAP_RESULT(inst->contains_valid_subchapter_text());
  UNWRAP_RESULT(inst->extract_subchapter_num());

  return Result::ok(inst.release(), deleter_SubChapter);
}

Result* extract_subchapter(uint32_t page, fz_context* ctx, fz_document* doc, void* shared) { return Result::ok(NULL, NULL); }
