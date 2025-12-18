#pragma once

#include <fitz.h>
#include <generated_page_types.h>
#include <vector>
#include <memory>

/**
 * Context for a specific page.
 */
struct PageContext
{
    const std::shared_ptr<const std::vector<KnownObject>> previous_pages; // 16 align
    const std::shared_ptr<const std::vector<fz_stext_page *>> text;       // 16 align
    const int32_t page_num;                                               // 4 align
};