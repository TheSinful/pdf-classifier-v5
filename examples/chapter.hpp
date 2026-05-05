#pragma once

#include "object.hpp"
#include "util.hpp"
#include <memory>
#include <mupdf/fitz.h>
#include <shared/result.h>
#include <string>

inline constexpr int EXPECTED_UPPERCASE_CHAPTER_FREQUENCY = 1;
inline constexpr int EXPECTED_LOWERCASE_CHAPTER_FREQUENCY = 2;

class Chapter : public Object<true, true> {
public:
  Chapter(fz_context* ctx, fz_document* doc, uint32_t page) : Object(ctx, doc, page) {}

  Result* contains_valid_chapter_text();
  Result* extract_chapter_number();

  std::string chapter_number;
};

void deleter_Chapter(void* p);
Result* classify_chapter(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract_chapter(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);
