#include "color_page.hpp"
#include <cstdlib>

namespace {

/// Drops the sampling pixmap on every exit path out of `ColorPage::sample()`.
struct PixmapGuard {
  fz_context* ctx;
  fz_pixmap* pix;

  ~PixmapGuard() {
    if (pix) {
      fz_drop_pixmap(ctx, pix);
    }
  }
};

int channel_delta(uint8_t a, uint8_t b) { return std::abs(static_cast<int>(a) - static_cast<int>(b)); }

} // namespace

bool within_tolerance(Rgb a, Rgb b) {
  return channel_delta(a.r, b.r) <= COLOR_TOLERANCE && channel_delta(a.g, b.g) <= COLOR_TOLERANCE &&
         channel_delta(a.b, b.b) <= COLOR_TOLERANCE;
}

std::string rgb_to_string(Rgb c) {
  return std::format("rgb({}, {}, {})", static_cast<int>(c.r), static_cast<int>(c.g), static_cast<int>(c.b));
}

ColorPage::ColorPage(fz_context* ctx, fz_document* doc, uint32_t page_num)
    : color{0, 0, 0}, page_num(page_num), ctx(ctx), doc(doc), page(nullptr), sampled(false) {
  // A page that will not load is a failed classification, not a crashed worker:
  // the failure is carried to sample() rather than thrown across the FFI edge.
  fz_try(ctx) { page = fz_load_page(ctx, doc, static_cast<int>(page_num)); }
  fz_catch(ctx) {
    page = nullptr;
    load_error = fz_caught_message(ctx);
  }
}

ColorPage::~ColorPage() {
  if (page) {
    fz_drop_page(ctx, page);
  }
}

Result* ColorPage::sample() {
  if (sampled) {
    return Result::ok(NULL, NULL);
  }

  PDF_ASSERT(page == nullptr, "failed to load page {}: {}", page_num,
             load_error.empty() ? "page could not be loaded" : load_error);

  // volatile: written inside fz_try (a setjmp target) and read after it.
  fz_pixmap* volatile pix = NULL;

  fz_try(ctx) {
    fz_rect bounds = fz_bound_page(ctx, page);
    float width = bounds.x1 - bounds.x0;
    float height = bounds.y1 - bounds.y0;

    if (width > 0.0f && height > 0.0f) {
      fz_matrix ctm = fz_scale(static_cast<float>(SAMPLE_DIM) / width, static_cast<float>(SAMPLE_DIM) / height);
      pix = fz_new_pixmap_from_page(ctx, page, ctm, fz_device_rgb(ctx), 0);
    }
  }
  fz_catch(ctx) { return Result::fail(std::format("failed to render page {}: {}", page_num, fz_caught_message(ctx))); }

  PixmapGuard guard{ctx, pix};

  PDF_ASSERT(pix == NULL, "page {} has an empty bounding box", page_num);
  PDF_ASSERT(pix->n < 3, "page {} rendered with {} components, expected at least 3 (rgb)", page_num,
             static_cast<int>(pix->n));

  const int low = SAMPLE_MARGIN;
  const int high_x = pix->w - SAMPLE_MARGIN;
  const int high_y = pix->h - SAMPLE_MARGIN;

  PDF_ASSERT(high_x <= low || high_y <= low, "page {} rendered to {}x{}, too small to sample with a {}px margin",
             page_num, pix->w, pix->h, SAMPLE_MARGIN);

  const unsigned char* origin = pix->samples + static_cast<ptrdiff_t>(low) * pix->stride +
                                static_cast<ptrdiff_t>(low) * pix->n;
  const Rgb first{origin[0], origin[1], origin[2]};

  for (int y = low; y < high_y; ++y) {
    const unsigned char* row = pix->samples + static_cast<ptrdiff_t>(y) * pix->stride;
    for (int x = low; x < high_x; ++x) {
      const unsigned char* px = row + static_cast<ptrdiff_t>(x) * pix->n;
      const Rgb here{px[0], px[1], px[2]};

      PDF_ASSERT(!within_tolerance(here, first), "page {} is not monochromatic: pixel ({}, {}) is {} but ({}, {}) is {}",
                 page_num, x, y, rgb_to_string(here), low, low, rgb_to_string(first));
    }
  }

  color = first;
  sampled = true;
  return Result::ok(NULL, NULL);
}

Result* ColorPage::expect(Rgb expected, const char* class_name) {
  UNWRAP_RESULT(sample());

  PDF_ASSERT(!within_tolerance(color, expected), "page {} is {}, which is not the {} color {}", page_num,
             rgb_to_string(color), class_name, rgb_to_string(expected));

  return Result::ok(NULL, NULL);
}

void deleter_ColorPage(void* p) { delete static_cast<ColorPage*>(p); }
