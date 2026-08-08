#include "color_page.hpp"
#include <cstdlib>

int channel_delta(uint8_t a, uint8_t b) { return std::abs(static_cast<int>(a) - static_cast<int>(b)); }

bool within_tolerance(Rgb a, Rgb b)
{
    return channel_delta(a.r, b.r) <= COLOR_TOLERANCE && channel_delta(a.g, b.g) <= COLOR_TOLERANCE &&
           channel_delta(a.b, b.b) <= COLOR_TOLERANCE;
}

std::string rgb_to_string(Rgb c)
{
    return std::format("rgb({}, {}, {})", static_cast<int>(c.r), static_cast<int>(c.g), static_cast<int>(c.b));
}

Result *ColorPage::sample(Attached &att)
{
    if (sampled)
    {
        return Result::ok(NULL, NULL);
    }

    // volatile: written inside fz_try (a setjmp target) and read after it.
    fz_pixmap *volatile pix = NULL;

    fz_try(att.raw_ctx())
    {
        fz_rect bounds = fz_bound_page(att.raw_ctx(), att.raw_page());
        float width = bounds.x1 - bounds.x0;
        float height = bounds.y1 - bounds.y0;

        if (width > 0.0f && height > 0.0f)
        {
            fz_matrix ctm = fz_scale(static_cast<float>(SAMPLE_DIM) / width, static_cast<float>(SAMPLE_DIM) / height);
            pix = fz_new_pixmap_from_page(att.raw_ctx(), att.raw_page(), ctm, fz_device_rgb(att.raw_ctx()), 0);
        }
    }
    fz_catch(att.raw_ctx()) { return Result::fail(std::format("failed to render page {}: {}", page_num, fz_caught_message(att.raw_ctx()))); }

    FzPixmap guard;
    try
    {
        fz_rect bounds = fz_bound_page(att.raw_ctx(), att.raw_page());
        float width = bounds.x1 - bounds.x0;
        float height = bounds.y1 - bounds.y0;

        if (width > 0.0f && height > 0.0f)
        {
            fz_matrix ctm = fz_scale(static_cast<float>(SAMPLE_DIM) / width, static_cast<float>(SAMPLE_DIM) / height);
            guard = FzPixmap::make(att.raw_ctx(), fz_new_pixmap_from_page, att.raw_page(), ctm, fz_device_rgb(att.raw_ctx()), 0);
        }
    }
    catch (const std::exception &e)
    {
        return Result::fail(std::format("failed to render page {}: {}", page_num, fz_caught_message(att.raw_ctx())));
    }

    ASSERT_RAW_RESULT(pix == NULL, "page {} has an empty bounding box", page_num);
    ASSERT_RAW_RESULT(pix->n < 3, "page {} rendered with {} components, expected at least 3 (rgb)", page_num,
               static_cast<int>(pix->n));

    const int low = SAMPLE_MARGIN;
    const int high_x = pix->w - SAMPLE_MARGIN;
    const int high_y = pix->h - SAMPLE_MARGIN;

    ASSERT_RAW_RESULT(high_x <= low || high_y <= low, "page {} rendered to {}x{}, too small to sample with a {}px margin",
               page_num, pix->w, pix->h, SAMPLE_MARGIN);

    const unsigned char *origin = pix->samples + static_cast<ptrdiff_t>(low) * pix->stride +
                                  static_cast<ptrdiff_t>(low) * pix->n;
    const Rgb first{origin[0], origin[1], origin[2]};

    for (int y = low; y < high_y; ++y)
    {
        const unsigned char *row = pix->samples + static_cast<ptrdiff_t>(y) * pix->stride;
        for (int x = low; x < high_x; ++x)
        {
            const unsigned char *px = row + static_cast<ptrdiff_t>(x) * pix->n;
            const Rgb here{px[0], px[1], px[2]};

            ASSERT_RAW_RESULT(!within_tolerance(here, first), "page {} is not monochromatic: pixel ({}, {}) is {} but ({}, {}) is {}",
                       page_num, x, y, rgb_to_string(here), low, low, rgb_to_string(first));
        }
    }

    color = first;
    sampled = true;
    return Result::ok(NULL, NULL);
}

Result *ColorPage::expect(Attached &att, Rgb expected, const char *class_name)
{
    UNWRAP_RAW_RESULT(sample(att));

    ASSERT_RAW_RESULT(!within_tolerance(color, expected), "page {} is {}, which is not the {} color {}", page_num,
                      rgb_to_string(color), class_name, rgb_to_string(expected));

    return Result::ok(NULL, NULL);
}

void deleter_ColorPage(void *p) { delete static_cast<ColorPage *>(p); }
