#include "../string_utils.hpp"
#include "../wrappers.hpp"
#include "pdf_fixture.hpp"

#include <gtest/gtest.h>

// ---------------------------------------------------------------------------
// Pure string logic - no MuPDF, no document.
// ---------------------------------------------------------------------------

TEST(Levenshtein, IdenticalStringsAreZero)
{
    EXPECT_EQ(levenshtein_distance("chapter", "chapter"), 0);
    EXPECT_EQ(levenshtein_distance("", ""), 0);
}

TEST(Levenshtein, EmptyAgainstNonEmptyIsLength)
{
    EXPECT_EQ(levenshtein_distance("", "abcd"), 4);
    EXPECT_EQ(levenshtein_distance("abcd", ""), 4);
}

TEST(Levenshtein, CountsSingleEdits)
{
    EXPECT_EQ(levenshtein_distance("chapter", "chapten"), 1); // substitution
    EXPECT_EQ(levenshtein_distance("chapter", "chapters"), 1); // insertion
    EXPECT_EQ(levenshtein_distance("chapter", "chaptr"), 1);   // deletion
}

TEST(Levenshtein, IsSymmetric)
{
    EXPECT_EQ(levenshtein_distance("kitten", "sitting"),
              levenshtein_distance("sitting", "kitten"));
    EXPECT_EQ(levenshtein_distance("kitten", "sitting"), 3);
}

TEST(FrequencyOf, ExactMatchesWithZeroErrors)
{
    EXPECT_EQ(frequency_of("Chapter", "Chapter one, Chapter two", 0), 2);
    EXPECT_EQ(frequency_of("Chapter", "nothing here", 0), 0);
}

TEST(FrequencyOf, ToleratesEditsUpToMaxErrors)
{
    // "Chaptor" is one substitution away.
    EXPECT_EQ(frequency_of("Chapter", "Chaptor", 0), 0);
    EXPECT_GE(frequency_of("Chapter", "Chaptor", 1), 1);
}

TEST(FrequencyOf, DegenerateInputsReturnZero)
{
    EXPECT_EQ(frequency_of("", "anything", 0), 0);
    EXPECT_EQ(frequency_of("longer than haystack", "short", 0), 0);
}

// Case matters - Diagram relies on "Chapter" and "CHAPTER" counting separately.
TEST(FrequencyOf, IsCaseSensitive)
{
    EXPECT_EQ(frequency_of("CHAPTER", "Chapter one", 0), 0);
}

TEST(ContainsText, FindsSubstring)
{
    EXPECT_TRUE(contains_text("Fig", "Fig 3: caption"));
    EXPECT_FALSE(contains_text("Table", "Fig 3: caption"));
    EXPECT_TRUE(contains_text("", "anything")) << "empty needle is always found";
}

// ---------------------------------------------------------------------------
// compress_text
// ---------------------------------------------------------------------------

namespace
{
    PdfText run(std::string text)
    {
        PdfText t;
        t.text = std::move(text);
        t.font_name = "Helvetica";
        t.font_size = 10.0f;
        t.bbox = fz_empty_rect;
        return t;
    }
}

TEST(CompressText, EmptyInputIsEmptyString)
{
    EXPECT_EQ(compress_text({}), "");
}

TEST(CompressText, JoinsRunsWithNewlines)
{
    EXPECT_EQ(compress_text({run("Fig 3:"), run("A caption")}), "Fig 3:\nA caption\n");
}

// Pins current behaviour: every run is terminated, so the output ends with a
// newline rather than using '\n' purely as a separator.
TEST(CompressText, TerminatesRatherThanSeparates)
{
    const std::string out = compress_text({run("only")});

    EXPECT_EQ(out, "only\n");
    ASSERT_FALSE(out.empty());
    EXPECT_EQ(out.back(), '\n');
}

TEST(CompressText, PreservesRunOrder)
{
    EXPECT_EQ(compress_text({run("a"), run("b"), run("c")}), "a\nb\nc\n");
}

// A run boundary is a font/size change, not a line break, so a style change
// mid-word splits that word across a newline. This documents the consequence:
// text that reads as one word on the page is no longer findable as one.
TEST(CompressText, StyleChangeMidWordBreaksSubstringSearch)
{
    const std::string out = compress_text({run("Chap"), run("ter")});

    EXPECT_EQ(out, "Chap\nter\n");
    EXPECT_FALSE(contains_text("Chapter", out));
    EXPECT_EQ(frequency_of("Chapter", out, 0), 0);
}

// ---------------------------------------------------------------------------
// as_string / IntoString
// ---------------------------------------------------------------------------

TEST(AsString, AcceptsEveryDeclaredOverload)
{
    EXPECT_EQ(as_string(std::string("owned")), "owned");
    EXPECT_EQ(as_string(std::string_view("view")), "view");
    EXPECT_EQ(as_string("literal"), "literal");

    const nlohmann::json j = {{"fig_num", 3}};
    EXPECT_EQ(as_string(j), j.dump());
}

TEST(AsString, ConceptAcceptsTheIntendedTypes)
{
    static_assert(IntoString<std::string>);
    static_assert(IntoString<std::string_view>);
    static_assert(IntoString<const char *>);
    static_assert(IntoString<nlohmann::json>);

    // A type nlohmann cannot build a json from is correctly rejected.
    static_assert(!IntoString<fz_rect>);
    SUCCEED();
}

// Documents a consequence of the as_string(const nlohmann::json&) overload:
// nlohmann::json has an implicit converting constructor from arithmetic types
// and containers, so IntoString admits far more than the name suggests.
// ExtractionResult::ok(42) compiles and yields "42".
TEST(AsString, ConceptAlsoAdmitsAnythingJsonConstructible)
{
    static_assert(IntoString<int>);
    static_assert(IntoString<bool>);
    static_assert(IntoString<double>);

    EXPECT_EQ(as_string(42), "42");
    EXPECT_EQ(as_string(true), "true");
}

// ---------------------------------------------------------------------------
// MuPDF-backed
// ---------------------------------------------------------------------------

class StringUtilsPdf : public PdfFixture
{
};

TEST_F(StringUtilsPdf, ExtractTextReturnsRunsForATextPage)
{
    FzPage page = FzPage::make(ctx, fz_load_page, doc, TEXT_PAGE);

    const std::vector<PdfText> runs = extract_text(ctx, page.get(), TEXT_PAGE);

    ASSERT_FALSE(runs.empty()) << "page " << TEXT_PAGE << " should carry text";

    for (const PdfText &r : runs)
    {
        EXPECT_FALSE(r.text.empty());
        EXPECT_GT(r.font_size, 0.0f);
    }
}

TEST_F(StringUtilsPdf, ExtractTextRejectsNullPage)
{
    EXPECT_THROW(extract_text(ctx, nullptr, TEXT_PAGE), FzError);
}

// Repeated calls must not accumulate - the original leaked an fz_stext_page
// every time, including on the error path.
TEST_F(StringUtilsPdf, ExtractTextIsRepeatable)
{
    FzPage page = FzPage::make(ctx, fz_load_page, doc, TEXT_PAGE);

    const size_t first = extract_text(ctx, page.get(), TEXT_PAGE).size();

    for (int i = 0; i < 5; ++i)
        EXPECT_EQ(extract_text(ctx, page.get(), TEXT_PAGE).size(), first);
}

TEST_F(StringUtilsPdf, CompressTextOverRealPageIsNonEmpty)
{
    FzPage page = FzPage::make(ctx, fz_load_page, doc, TEXT_PAGE);

    const std::string flat = compress_text(extract_text(ctx, page.get(), TEXT_PAGE));

    EXPECT_FALSE(flat.empty());
}

TEST_F(StringUtilsPdf, HasImageAnswersWithoutThrowing)
{
    FzPage page = FzPage::make(ctx, fz_load_page, doc, TEXT_PAGE);

    EXPECT_NO_THROW({ (void)has_image(ctx, page.get()); });
}

TEST_F(StringUtilsPdf, HasImageIsFalseForNullPage)
{
    EXPECT_FALSE(has_image(ctx, nullptr));
}

TEST_F(StringUtilsPdf, HasImageIsRepeatable)
{
    FzPage page = FzPage::make(ctx, fz_load_page, doc, TEXT_PAGE);

    const bool first = has_image(ctx, page.get());

    for (int i = 0; i < 5; ++i)
        EXPECT_EQ(has_image(ctx, page.get()), first);
}
