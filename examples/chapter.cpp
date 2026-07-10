#include "chapter.hpp"
#include <filesystem>
#include <fstream>
#include <regex>

using nlohmann::json;
using namespace std::filesystem;

Result* Chapter::contains_valid_chapter_text() {
  int upper_freq = frequency_of("CHAPTER", compressed_text, 1);
  int lower_freq = frequency_of("Chapter", compressed_text, 1);

  PDF_ASSERT(upper_freq != EXPECTED_UPPERCASE_CHAPTER_FREQUENCY,
             "found {} instances of 'CHAPTER'(uppercase), but expected {}", upper_freq, 1);

  PDF_ASSERT(lower_freq != EXPECTED_LOWERCASE_CHAPTER_FREQUENCY,
             "found {} instances of 'chapter'(lowercase), but expected {}", lower_freq, 2);

  for (PdfText& entry : extracted_text) {
    if (!contains_text("CHAPTER", entry.text))
      continue;

    if (!contains_text("Helvetica", entry.font_name))
      continue;

    // 10.4..11.0 font size range
    bool is_in_lower_bound = std::abs(entry.font_size - 10.4) <= 0.3;
    bool is_in_upper_bound = std::abs(entry.font_size - 11.0) <= 0.3;
    if (!is_in_lower_bound && !is_in_upper_bound)
      continue;

    return Result::ok(NULL, NULL);
  }

  return Result::fail("extracted text didn't have any entries with expected structure.");
}

Result* Chapter::extract_chapter_number() {
  // CHAPTER followed by X-Y format
  std::regex pattern = std::regex(R"(CHAPTER\s+(\d+-\d+))", std::regex_constants::icase);
  std::smatch match;

  if (std::regex_search(compressed_text, match, pattern)) {
    chapter_number = match[1].str();
    return Result::ok(NULL, NULL);
  }

  return Result::fail("no chapter number could be found within text of page.");
}

void Chapter::extract_expected_subchapters() {
  std::regex pattern = std::regex(std::format(R"({}-\d+)", chapter_number), std::regex_constants::icase);

  for (std::sregex_iterator it(compressed_text.begin(), compressed_text.end(), pattern); it != std::sregex_iterator{};
       ++it) {
    const std::smatch& match = *it;
    std::string matched_text = match.str();

    ExpectedSubchapter subchapter;
    subchapter.sub_chapter_num = matched_text;
    subchapter.path = std::format("./sub_chapter_{}", matched_text);

    expected_subchapters.emplace_back(subchapter);
  }
}

void deleter_Chapter(void* p) { delete static_cast<Chapter*>(p); }

Result* classify_chapter(uint32_t page, fz_context* ctx, fz_document* doc) {
  auto inst = std::make_unique<Chapter>(ctx, doc, page);

  UNWRAP_RESULT(inst->contains_valid_chapter_text());
  UNWRAP_RESULT(inst->extract_chapter_number());

  return Result::ok(inst.release(), deleter_Chapter);
}

Result* extract_chapter(uint32_t page, fz_context* ctx, fz_document* doc, void* shared) {
  Chapter* inst = static_cast<Chapter*>(shared);

  inst->extract_expected_subchapters();

  json data = nlohmann::json{{"chapter_num", inst->chapter_number}, {"sub_chapters", inst->expected_subchapters}};

  return json_to_payload(data);
}
