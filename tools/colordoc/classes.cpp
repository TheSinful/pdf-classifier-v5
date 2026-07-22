#include "classes.hpp"
#include "util.hpp"
#include <memory>

namespace {

/// Every class classifies identically: render the page, demand its color.
/// On success the sampled `ColorPage` becomes the shared payload so the
/// matching extract does not have to render a second time.
Result* classify_color(uint32_t page, fz_context* ctx, fz_document* doc, Rgb expected, const char* class_name) {
  auto inst = std::make_unique<ColorPage>(ctx, doc, page);

  UNWRAP_RESULT(inst->expect(expected, class_name));

  return Result::ok(inst.release(), deleter_ColorPage);
}

Result* extract_color(void* shared, const char* class_name) {
  ColorPage* inst = static_cast<ColorPage*>(shared);
  if (!inst) {
    return Result::fail(std::format("{} extraction received no shared payload", class_name));
  }

  return json_to_payload(nlohmann::json{
      {"page", inst->page_num},
      {"class", class_name},
      {"rgb", {static_cast<int>(inst->color.r), static_cast<int>(inst->color.g), static_cast<int>(inst->color.b)}},
  });
}

} // namespace

Result* classify_section(uint32_t page, fz_context* ctx, fz_document* doc) {
  return classify_color(page, ctx, doc, SECTION_COLOR, "section");
}

Result* extract_section(uint32_t page, fz_context* ctx, fz_document* doc, void* shared) {
  return extract_color(shared, "section");
}

Result* classify_subsection(uint32_t page, fz_context* ctx, fz_document* doc) {
  return classify_color(page, ctx, doc, SUBSECTION_COLOR, "subsection");
}

Result* extract_subsection(uint32_t page, fz_context* ctx, fz_document* doc, void* shared) {
  return extract_color(shared, "subsection");
}

Result* classify_figure(uint32_t page, fz_context* ctx, fz_document* doc) {
  return classify_color(page, ctx, doc, FIGURE_COLOR, "figure");
}

Result* extract_figure(uint32_t page, fz_context* ctx, fz_document* doc, void* shared) {
  return extract_color(shared, "figure");
}

Result* classify_caption(uint32_t page, fz_context* ctx, fz_document* doc) {
  return classify_color(page, ctx, doc, CAPTION_COLOR, "caption");
}

Result* extract_caption(uint32_t page, fz_context* ctx, fz_document* doc, void* shared) {
  return extract_color(shared, "caption");
}
