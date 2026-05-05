#pragma once

#include "object.hpp"
#include <shared/result.h>
#include <string>

class SubChapter : public Object<true> {
public:
  SubChapter(fz_context* ctx, fz_document* doc, uint32_t page) : Object(ctx, doc, page) {}

  Result* contains_valid_subchapter_text();
  Result* extract_subchapter_num();

  std::string subchapter_number;
};

inline constexpr int UPPERCASE_SUBCHAPTER_TEXT_FREQUENCY = 1;
inline constexpr int LOWERCASE_SUBCHAPTER_TEXT_FREQUENCY = 1;

void deleter_SubChapter(void* p);
Result* classify_subchapter(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract_subchapter(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);
