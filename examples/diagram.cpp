#include "diagram.hpp"
#include <memory>
#include <regex>

Result* Diagram::contains_valid_chapter_text() {
  if (frequency_of("Chapter", compressed_text, 1) == EXPECTED_LOWER_CHAPTER &&
      frequency_of("CHAPTER", compressed_text, 1) == EXPECTED_UPPER_CHAPTER) {
    return Result::ok(NULL, NULL);
  } else {
    return Result::fail("doesn't contain valid chapter text");
  }
}

Result* Diagram::contains_valid_figure_text() {
  std::regex pattern(R"(Fig\s+(\d+):\s*(.+))");
  std::smatch matches;

  if (std::regex_search(compressed_text, matches, pattern)) {
    fig_num = std::stoi(matches[1].str()); // int
    caption = matches[2].str();
    return Result::ok(NULL, NULL);
  } else {
    return Result::fail("failed to find figure text");
  }
}

Result* Diagram::contains_image() {
  if (has_image()) {
    return Result::ok(NULL, NULL);
  } else {
    return Result::fail("doesn't contain an image");
  }
}

void deleter_Diagram(void* p) { delete static_cast<Diagram*>(p); }

Result* classify_diagram(uint32_t page, fz_context* ctx, fz_document* doc) {
  auto inst = std::make_unique<Diagram>(ctx, doc, page);

  UNWRAP_RESULT(inst->contains_image());
  UNWRAP_RESULT(inst->contains_valid_chapter_text());
  UNWRAP_RESULT(inst->contains_valid_figure_text());

  return Result::ok(inst.release(), deleter_Diagram);
}

Result* extract_diagram(uint32_t page, fz_context* ctx, fz_document* doc, void* shared) { return Result::ok(NULL, NULL); }
