#pragma once

#include "object.hpp"
#include "util.hpp"
#include <mupdf/fitz.h>
#include <shared/result.h>

class Diagram : public Object<true, true> {
public:
  Diagram(fz_context* ctx, fz_document* doc, uint32_t page) : Object(ctx, doc, page) {}

  Result* contains_valid_chapter_text();
  Result* contains_valid_figure_text();
  Result* contains_image();

  int fig_num;
  std::string caption;
};

inline constexpr int EXPECTED_UPPER_CHAPTER = 0;
inline constexpr int EXPECTED_LOWER_CHAPTER = 1;

void deleter_Diagram(void* p);
Result* classify_diagram(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract_diagram(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);
