#pragma once

#include <gtest/gtest.h>
#include <mupdf/fitz.h>

#include <filesystem>

// Shared fixture for tests that need a real document. Skips rather than fails
// when the test PDF is absent, so the pure-logic suites still run.
class PdfFixture : public ::testing::Test
{
protected:
    fz_context *ctx = nullptr;
    fz_document *doc = nullptr;

    // Page 0 is known to carry extractable text (see examples/tests/test_object.cpp).
    static constexpr uint32_t TEXT_PAGE = 0;

    void SetUp() override
    {
        const std::filesystem::path pdf{TEST_PDF_PATH};

        if (!std::filesystem::exists(pdf))
            GTEST_SKIP() << "test document not found at " << pdf.string();

        ctx = fz_new_context(nullptr, nullptr, FZ_STORE_DEFAULT);
        ASSERT_NE(ctx, nullptr);

        fz_register_document_handlers(ctx);
        doc = fz_open_document(ctx, pdf.string().c_str());
        ASSERT_NE(doc, nullptr);
    }

    void TearDown() override
    {
        if (doc)
            fz_drop_document(ctx, doc);
        if (ctx)
            fz_drop_context(ctx);
    }
};
