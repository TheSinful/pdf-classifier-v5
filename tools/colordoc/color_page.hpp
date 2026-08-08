#pragma once

#include "palette.hpp"
#include "util.hpp"
#include <cstdint>
#include <mupdf/fitz.h>
#include <shared/result.h>
#include <string>
#include <pdf_classifier_lib/object.hpp>

/// Side of the square pixmap a page is rendered down to before sampling.
/// Nothing on a colordoc page varies spatially, so this only needs to be large
/// enough that a page which *isn't* a flat fill is detectable.
inline constexpr int SAMPLE_DIM = 16;

/// Pixels ignored on every edge of that pixmap. The fill bleeds past the trim,
/// but the rasteriser can still blend the outermost row against the page
/// boundary; the margin keeps that out of the sample.
inline constexpr int SAMPLE_MARGIN = 2;

bool within_tolerance(Rgb a, Rgb b);
std::string rgb_to_string(Rgb c);

/// A colordoc page is one flat monochromatic fill, so its entire classification
/// surface is a single RGB triple.
///
/// `sample()` renders the page and reads that triple back, refusing to average:
/// a page that is *not* uniform is a document-generator bug, and is reported as
/// a failure rather than silently resolved to a nearby class. That refusal is
/// what keeps "this page is class X" objectively decidable, which is the whole
/// point of the tool.
class ColorPage
{
public:
    ColorPage() = default;
    ~ColorPage() = default;

    ColorPage(const ColorPage &) = delete;
    ColorPage &operator=(const ColorPage &) = delete;

    /// Renders and reads this page's color into `color`. Idempotent.
    Result *sample(Attached &att);

    /// `sample()`, then assert the result is `expected`. `class_name` only ever
    /// appears in the failure reason.
    Result *expect(Attached &att, Rgb expected, const char *class_name);

    Rgb color;
    uint32_t page_num;

private:
    std::string load_error;
    bool sampled;
};

void deleter_ColorPage(void *p);
