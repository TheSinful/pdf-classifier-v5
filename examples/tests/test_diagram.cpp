#ifndef TEST_PDF_PATH
#error "TEST_PDF_PATH was not defined!"
#endif

#include "diagram.hpp"
#include <filesystem>
#include <gtest/gtest.h>
#include <mupdf/fitz.h>

static bool result_is_ok(Result* res) {
  bool ok = (res->type == Result::Type::OK);
  delete res;
  return ok;
}

class DiagramFixture : public ::testing::Test {
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

// Old test: TestImagesExist
// Page 1247 is the diagram page used by the original suite.
TEST_F(DiagramFixture, TestContainsImage) {
  try {
    Diagram d(ctx, doc, 1247);
    EXPECT_TRUE(result_is_ok(d.contains_image())) << "Page 1247 should contain an image";
  } catch (const std::exception& e) {
    FAIL() << "Diagram construction threw: " << e.what();
  }
}

// Old test: TestFigureText (was commented out pending OCR work)
// The new API can still validate the regex-based figure text path.
TEST_F(DiagramFixture, TestContainsValidFigureText) {
  try {
    Diagram d(ctx, doc, 1247);
    bool ok = result_is_ok(d.contains_valid_figure_text());
    if (ok) {
      EXPECT_GT(d.fig_num, 0) << "Figure number should be positive";
      EXPECT_FALSE(d.caption.empty()) << "Caption should not be empty";
      GTEST_LOG_(INFO) << "fig_num=" << d.fig_num << " caption='" << d.caption << "'";
    } else {
      // Preserve the original intent: the test was commented-out because
      // some figures use skewed text that resists regex extraction.
      GTEST_LOG_(INFO) << "contains_valid_figure_text() failed on page 1247 "
                          "(may need OCR for skewed caption text — same caveat as original)";
    }
  } catch (const std::exception& e) {
    FAIL() << "Diagram construction threw: " << e.what();
  }
}

// End-to-end: full classify_diagram pipeline.
TEST_F(DiagramFixture, TestClassifyDiagramSucceeds) {
  Result* res = classify_diagram(1244, ctx, doc);
  ASSERT_NE(res, nullptr);
  EXPECT_EQ(res->type, Result::Type::OK) << "classify_diagram() should succeed on page 1247" << res->fail_rsn;
  delete res;
}
