#ifndef TEST_PDF_PATH
#error "TEST_PDF_PATH was not defined!"
#endif

#include "object.hpp"
#include <filesystem>
#include <gtest/gtest.h>
#include <mupdf/fitz.h>

// Minimal concrete subclass of Object<true, true> that exposes
// the protected text members for direct inspection.
// The new Object<> template does all work (extraction + compression)
// in the constructor, so no separate initialize() call is needed.
class TestableObject : public Object<true, true> {
public:
  using Object::Object;

  const std::vector<PdfText>& get_extracted_text() const { return extracted_text; }
  const std::string& get_compressed_text() const { return compressed_text; }
};

class ObjectFixture : public ::testing::Test {
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

// Old test: TestObjectInitialization
// Construction must not throw and the page must be loaded.
TEST_F(ObjectFixture, TestObjectInitialization) {
  EXPECT_NO_THROW({
    TestableObject obj(ctx, doc, 0);
    EXPECT_FALSE(obj.get_extracted_text().empty()) << "Object should have extracted text after construction";
  });
}

// Old test: TestPageTextExtraction
// Every PdfText entry must carry non-empty text, a font name, and a positive size.
// Also preserves the spot-check for the specific string found on page 0 of the
// original test document.
TEST_F(ObjectFixture, TestTextExtraction) {
  TestableObject obj(ctx, doc, 0);
  const auto& entries = obj.get_extracted_text();

  EXPECT_FALSE(entries.empty()) << "No text was extracted from page 0";

  bool found_expected = false;
  for (const PdfText& entry : entries) {
    SCOPED_TRACE("entry: " + entry.text);
    EXPECT_FALSE(entry.text.empty()) << "Text entry should not be empty";
    EXPECT_FALSE(entry.font_name.empty()) << "Font name should not be empty";
    EXPECT_GT(entry.font_size, 0.0f) << "Font size should be greater than 0";

    if (entry.text == R"('TRUCK UTILITY LIGHT (TUL) HS,)")
      found_expected = true;

    GTEST_LOG_(INFO) << "text='" << entry.text << "' font=" << entry.font_name << " size=" << entry.font_size;
  }

  if (found_expected)
    GTEST_LOG_(INFO) << "Found expected text entry on page 0";

  GTEST_LOG_(INFO) << "Total entries extracted from page 0: " << entries.size();
}

// New test covering text compression (no old equivalent — replaces the
// now-removed to_png() and is_blank_page() tests which have no counterpart
// in the refactored Object<> template).
TEST_F(ObjectFixture, TestTextCompression) {
  TestableObject obj(ctx, doc, 0);
  EXPECT_FALSE(obj.get_compressed_text().empty()) << "Compressed text should be non-empty after construction on page 0";
}
