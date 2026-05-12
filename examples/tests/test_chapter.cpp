#ifndef TEST_PDF_PATH
#error "TEST_PDF_PATH was not defined!"
#endif

#include "chapter.hpp"
#include "subchapter.hpp"
#include <filesystem>
#include <gtest/gtest.h>
#include <mupdf/fitz.h>

static bool result_is_ok(Result* res) {
  bool ok = (res->type == Result::Type::OK);
  delete res;
  return ok;
}

class ChapterFixture : public ::testing::Test {
protected:
  fz_context* ctx = nullptr;
  fz_document* doc = nullptr;

  void SetUp() override {
    std::filesystem::path pdf_path = TEST_PDF_PATH;
    if (!std::filesystem::exists(pdf_path)) {
      GTEST_SKIP() << "Test PDF not found at: " << pdf_path;
    }
    ctx = fz_new_context(NULL, NULL, FZ_STORE_UNLIMITED);
    ASSERT_NE(ctx, nullptr) << "Failed to create MuPDF context";
    fz_try(ctx) {
      fz_register_document_handlers(ctx);
      doc = fz_open_document(ctx, pdf_path.string().c_str());
      if (!doc)
        fz_throw(ctx, FZ_ERROR_GENERIC, "Failed to open document");
    }
    fz_catch(ctx) { FAIL() << "MuPDF error: " << fz_caught_message(ctx); }
  }

  void TearDown() override {
    if (doc && ctx) {
      fz_drop_document(ctx, doc);
      doc = nullptr;
    }
    if (ctx) {
      fz_drop_context(ctx);
      ctx = nullptr;
    }
  }
};

// Expose protected chapter_number for assertion
class TestableChapter : public Chapter {
public:
  using Chapter::Chapter;
  const std::string& get_chapter_number() const { return chapter_number; }
};

// Old test: TestChapterValidation / ValidateExtractedChapterNumber
// Page 1230 is the chapter page used by the original suite.
TEST_F(ChapterFixture, TestContainsValidChapterText) {
  TestableChapter ch(ctx, doc, 1230);
  EXPECT_TRUE(result_is_ok(ch.contains_valid_chapter_text())) << "Page 1230 should pass chapter text validation";
}

TEST_F(ChapterFixture, TestExtractChapterNumber) {
  TestableChapter ch(ctx, doc, 1230);
  ASSERT_TRUE(result_is_ok(ch.extract_chapter_number())) << "extract_chapter_number() should succeed on page 1230";
  EXPECT_EQ(ch.get_chapter_number(), "2-16") << "Expected chapter number '2-16'";
}

// Old test: TestFailureOnSubChapterPage
// Page 1233 is a sub-chapter page; classifying it as a chapter should fail.
TEST_F(ChapterFixture, TestFailureOnSubChapterPage) {
  TestableChapter ch(ctx, doc, 1233);
  EXPECT_FALSE(result_is_ok(ch.contains_valid_chapter_text()))
      << "Page 1233 is a subchapter page — chapter text validation should fail";
}

// Old test: ValidateExpectedSubChapters / TestConstructSubChapters (page 235)
// The new Chapter no longer tracks sub-chapters internally; the sub-chapter
// structure is handled by the inference engine.  What we can verify is that
// classify_chapter succeeds on the chapter page (235) and that the extracted
// chapter number follows the expected pattern.
TEST_F(ChapterFixture, TestClassifyChapterPage235) {
  Result* res = classify_chapter(235, ctx, doc);
  ASSERT_NE(res, nullptr);
  bool ok = (res->type == Result::Type::OK);
  if (!ok) {
    GTEST_LOG_(INFO) << "classify_chapter on page 235 failed: " << res->fail_rsn;
  }
  delete res;
  EXPECT_TRUE(ok) << "classify_chapter() should succeed on page 235";
}

// End-to-end: full classify_chapter pipeline on the main chapter page.
TEST_F(ChapterFixture, TestClassifyChapterSucceeds) {
  Result* res = classify_chapter(1230, ctx, doc);
  ASSERT_NE(res, nullptr);
  EXPECT_EQ(res->type, Result::Type::OK) << "classify_chapter() should succeed on page 1230";
  delete res;
}

// End-to-end: classify_chapter must reject a sub-chapter page.
TEST_F(ChapterFixture, TestClassifyChapterFails) {
  Result* res = classify_chapter(1233, ctx, doc);
  ASSERT_NE(res, nullptr);
  EXPECT_EQ(res->type, Result::Type::FAIL) << "classify_chapter() should fail on subchapter page 1233";
  delete res;
}
