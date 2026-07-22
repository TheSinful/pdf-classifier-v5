#include "colordoc_fixture.hpp"
#include "color_page.hpp"

using ColorPageTest = ColordocFixture;

// The tolerance comparison is the one piece of arithmetic the whole tool rests
// on, so it is pinned independently of any document.
TEST(ColorPageUnit, WithinToleranceAcceptsExactAndNearMatches) {
  EXPECT_TRUE(within_tolerance(SECTION_COLOR, SECTION_COLOR));

  const Rgb nudged{static_cast<uint8_t>(SECTION_COLOR.r + COLOR_TOLERANCE), SECTION_COLOR.g, SECTION_COLOR.b};
  EXPECT_TRUE(within_tolerance(SECTION_COLOR, nudged)) << "a delta of exactly COLOR_TOLERANCE should still match";
}

TEST(ColorPageUnit, WithinToleranceRejectsDistinctPaletteEntries) {
  EXPECT_FALSE(within_tolerance(SECTION_COLOR, SUBSECTION_COLOR));
  EXPECT_FALSE(within_tolerance(FIGURE_COLOR, CAPTION_COLOR));
  EXPECT_FALSE(within_tolerance(SECTION_COLOR, BLANK_COLOR));

  const Rgb past{static_cast<uint8_t>(SECTION_COLOR.r + COLOR_TOLERANCE + 1), SECTION_COLOR.g, SECTION_COLOR.b};
  EXPECT_FALSE(within_tolerance(SECTION_COLOR, past)) << "a delta of COLOR_TOLERANCE+1 must not match";
}

TEST(ColorPageUnit, RgbToStringFormatsChannelsAsNumbers) {
  // uint8_t formats as a character if it reaches std::format unwidened; this
  // catches that regression, which would silently corrupt every failure reason.
  EXPECT_EQ(rgb_to_string(Rgb{65, 66, 67}), "rgb(65, 66, 67)");
}

// Sampling must recover the exact fill the generator painted, on every page.
TEST_F(ColorPageTest, SamplesEveryPageAsItsGroundTruthColor) {
  for (size_t page = 0; page < pages.size(); ++page) {
    ColorPage subject(ctx, doc, static_cast<uint32_t>(page));

    EXPECT_TRUE(is_ok(subject.sample())) << "page " << page << " (" << pages[page] << ") failed to sample";

    const Rgb expected = expected_color(pages[page]);
    EXPECT_TRUE(within_tolerance(subject.color, expected))
        << "page " << page << " is " << pages[page] << ", expected " << rgb_to_string(expected) << " but sampled "
        << rgb_to_string(subject.color);
  }
}

TEST_F(ColorPageTest, SampleIsIdempotent) {
  ColorPage subject(ctx, doc, 0);

  ASSERT_TRUE(is_ok(subject.sample()));
  const Rgb first = subject.color;

  ASSERT_TRUE(is_ok(subject.sample())) << "re-sampling should be a no-op, not a re-render";
  EXPECT_TRUE(within_tolerance(first, subject.color));
}

TEST_F(ColorPageTest, PageNumIsPreserved) {
  const uint32_t target = static_cast<uint32_t>(pages.size() - 1);
  ColorPage subject(ctx, doc, target);

  ASSERT_TRUE(is_ok(subject.sample()));
  EXPECT_EQ(subject.page_num, target);
}

// A page past the end of the document must come back as a clean failure. If the
// constructor threw instead, the exception would cross the FFI boundary and
// take a worker thread with it.
TEST_F(ColorPageTest, OutOfRangePageFailsInsteadOfThrowing) {
  const uint32_t past_end = static_cast<uint32_t>(pages.size() + 16);

  ASSERT_NO_THROW({
    ColorPage subject(ctx, doc, past_end);
    EXPECT_TRUE(is_fail(subject.sample())) << "page " << past_end << " does not exist and must not sample";
  });
}
