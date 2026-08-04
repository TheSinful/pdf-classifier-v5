#include "../object.hpp"
#include "pdf_fixture.hpp"

#include <gtest/gtest.h>

#include <string>

// ---------------------------------------------------------------------------
// Attached - the per-call capability handed to user objects.
// ---------------------------------------------------------------------------

class AttachedTest : public PdfFixture
{
};

TEST_F(AttachedTest, ExposesTheCallsResources)
{
    Attached att(ctx, doc, TEXT_PAGE);

    EXPECT_EQ(att.raw_ctx(), ctx);
    EXPECT_EQ(att.raw_doc(), doc);
    EXPECT_NE(att.raw_page(), nullptr);
    EXPECT_EQ(att.page_num(), TEXT_PAGE);
}

// MuPDF keeps a list of open pages per document, so two live Attacheds on the
// same page share one refcounted fz_page rather than getting distinct objects.
// Ownership is still per-Attached - each holds its own reference and drops it.
TEST_F(AttachedTest, ConcurrentAttachmentsShareARefcountedPage)
{
    Attached first(ctx, doc, TEXT_PAGE);
    Attached second(ctx, doc, TEXT_PAGE);

    ASSERT_NE(first.raw_page(), nullptr);
    ASSERT_NE(second.raw_page(), nullptr);
    EXPECT_EQ(first.raw_page(), second.raw_page());
}

// The sequential case is what actually happens: classify's Attached is long
// dead before extract's is built. Note the addresses may well match anyway,
// because MuPDF is free to reuse the freed slot - which is exactly why a
// raw_page() pointer stashed across calls fails intermittently rather than
// reliably.
TEST_F(AttachedTest, EachCallGetsAUsablePageAfterThePreviousIsGone)
{
    {
        Attached first(ctx, doc, TEXT_PAGE);
        ASSERT_NE(first.raw_page(), nullptr);
    }

    Attached second(ctx, doc, TEXT_PAGE);

    ASSERT_NE(second.raw_page(), nullptr);
    EXPECT_FALSE(second.extract_text().empty()) << "the page is genuinely usable";
}

TEST_F(AttachedTest, ForwardingHelpersMatchTheFreeFunctions)
{
    Attached att(ctx, doc, TEXT_PAGE);

    EXPECT_EQ(att.has_image(), has_image(ctx, att.raw_page()));
    EXPECT_EQ(att.extract_text().size(),
              extract_text(ctx, att.raw_page(), TEXT_PAGE).size());
}

TEST_F(AttachedTest, PropagatesFzErrorForAnInvalidPage)
{
    EXPECT_THROW(Attached(ctx, doc, 99999999), FzError);
}

// Repeatedly constructing and destroying must not accumulate pages - this is
// the per-call lifecycle the whole design rests on.
TEST_F(AttachedTest, IsRepeatable)
{
    for (int i = 0; i < 10; ++i)
    {
        Attached att(ctx, doc, TEXT_PAGE);
        EXPECT_NE(att.raw_page(), nullptr);
    }

    SUCCEED();
}

// ---------------------------------------------------------------------------
// allocate() - resources registered with Attached die with it.
// ---------------------------------------------------------------------------

namespace
{
    struct Tracked
    {
        int id;
    };

    int g_tracked_drops = 0;

    void drop_tracked(fz_context *, Tracked *t)
    {
        ++g_tracked_drops;
        delete t;
    }

    Tracked *new_tracked(fz_context *, int id) { return new Tracked{id}; }

    using TrackedOwned = FzOwned<Tracked, drop_tracked>;
}

class AllocateTest : public PdfFixture
{
protected:
    void SetUp() override
    {
        PdfFixture::SetUp();
        g_tracked_drops = 0;
    }
};

TEST_F(AllocateTest, ReturnsABorrowedPointer)
{
    Attached att(ctx, doc, TEXT_PAGE);

    Tracked *t = att.allocate<TrackedOwned>(new_tracked, 42);

    ASSERT_NE(t, nullptr);
    EXPECT_EQ(t->id, 42);
    EXPECT_EQ(g_tracked_drops, 0) << "still owned by Attached";
}

TEST_F(AllocateTest, DropsRegisteredResourcesWhenAttachedDies)
{
    {
        Attached att(ctx, doc, TEXT_PAGE);
        att.allocate<TrackedOwned>(new_tracked, 1);
        att.allocate<TrackedOwned>(new_tracked, 2);

        EXPECT_EQ(g_tracked_drops, 0);
    }

    EXPECT_EQ(g_tracked_drops, 2);
}

TEST_F(AllocateTest, BorrowedPointersStayValidAsMoreAreAllocated)
{
    Attached att(ctx, doc, TEXT_PAGE);

    Tracked *first = att.allocate<TrackedOwned>(new_tracked, 7);

    for (int i = 0; i < 64; ++i)
        att.allocate<TrackedOwned>(new_tracked, 100 + i);

    EXPECT_EQ(first->id, 7) << "growth moves the owners, not the resources";
}

TEST_F(AllocateTest, WorksWithRealMuPdfResources)
{
    Attached att(ctx, doc, TEXT_PAGE);

    fz_stext_options opts = {0};
    fz_stext_page *stext =
        att.allocate<FzSTextPage>(fz_new_stext_page_from_page, att.raw_page(), &opts);

    ASSERT_NE(stext, nullptr);
}

// ---------------------------------------------------------------------------
// DEFINE_OBJECT - the full shim, classify through extract.
// ---------------------------------------------------------------------------

namespace
{
    bool g_classify_fails = false;
    bool g_extract_fails = false;
    int g_objects_alive = 0;

    class TestObject : public Object
    {
    public:
        explicit TestObject(uint32_t page) : Object(static_cast<int>(page)) { ++g_objects_alive; }
        ~TestObject() override { --g_objects_alive; }

        ClassificationResult classify(Attached &att) override
        {
            if (g_classify_fails)
                return ClassificationResult::fail("classify was told to fail");

            // Plain data, computed under the call, surviving to extract.
            seen_page = att.page_num();
            return ClassificationResult::ok();
        }

        ExtractionResult extract(Attached &att) override
        {
            if (g_extract_fails)
                return ExtractionResult::fail("extract was told to fail");

            EXPECT_EQ(att.page_num(), seen_page) << "same page, second attachment";
            return ExtractionResult::ok(std::string("extracted:") + std::to_string(seen_page));
        }

        uint32_t seen_page = 9999;
    };
}

DEFINE_OBJECT(testobj, TestObject);

class DefineObjectTest : public PdfFixture
{
protected:
    void SetUp() override
    {
        PdfFixture::SetUp();
        g_classify_fails = false;
        g_extract_fails = false;
        g_objects_alive = 0;
    }
};

TEST_F(DefineObjectTest, ClassifySuccessYieldsTheObjectAsPayload)
{
    Result *res = classify_testobj(TEXT_PAGE, ctx, doc);

    ASSERT_NE(res, nullptr);
    EXPECT_EQ(res->type, Result::OK);
    ASSERT_NE(res->payload, nullptr);
    ASSERT_NE(res->deleter, nullptr);
    EXPECT_EQ(g_objects_alive, 1);

    auto *obj = static_cast<TestObject *>(res->payload);
    EXPECT_EQ(obj->seen_page, TEXT_PAGE) << "state set in classify survives";

    res->deleter(res->payload);
    delete res;

    EXPECT_EQ(g_objects_alive, 0);
}

TEST_F(DefineObjectTest, ClassifyFailureDestroysTheObject)
{
    g_classify_fails = true;

    Result *res = classify_testobj(TEXT_PAGE, ctx, doc);

    ASSERT_NE(res, nullptr);
    EXPECT_EQ(res->type, Result::FAIL);
    EXPECT_EQ(res->fail_rsn, "classify was told to fail");
    EXPECT_EQ(g_objects_alive, 0) << "the unique_ptr must not leak the object on failure";

    delete res;
}

TEST_F(DefineObjectTest, ExtractReadsTheStashedObject)
{
    Result *classified = classify_testobj(TEXT_PAGE, ctx, doc);
    ASSERT_EQ(classified->type, Result::OK);

    Result *extracted = extract_testobj(TEXT_PAGE, ctx, doc, classified->payload);

    ASSERT_NE(extracted, nullptr);
    EXPECT_EQ(extracted->type, Result::OK);
    ASSERT_NE(extracted->payload, nullptr);

    EXPECT_EQ(*static_cast<std::string *>(extracted->payload),
              "extracted:" + std::to_string(TEXT_PAGE));

    extracted->deleter(extracted->payload);
    delete extracted;

    classified->deleter(classified->payload);
    delete classified;
}

TEST_F(DefineObjectTest, ExtractFailurePropagatesTheReason)
{
    Result *classified = classify_testobj(TEXT_PAGE, ctx, doc);
    ASSERT_EQ(classified->type, Result::OK);

    g_extract_fails = true;
    Result *extracted = extract_testobj(TEXT_PAGE, ctx, doc, classified->payload);

    ASSERT_NE(extracted, nullptr);
    EXPECT_EQ(extracted->type, Result::FAIL);
    EXPECT_EQ(extracted->fail_rsn, "extract was told to fail");

    delete extracted;

    classified->deleter(classified->payload);
    delete classified;
}

// The payload must be safe to destroy without ever running extract - the
// deferral engine abandons speculative classifications routinely.
TEST_F(DefineObjectTest, AbandonedPayloadDestroysCleanly)
{
    Result *res = classify_testobj(TEXT_PAGE, ctx, doc);
    ASSERT_EQ(res->type, Result::OK);
    EXPECT_EQ(g_objects_alive, 1);

    res->deleter(res->payload);
    delete res;

    EXPECT_EQ(g_objects_alive, 0);
}

// Both calls attach independently, so a second classify on the same page must
// not be disturbed by the first having completed.
TEST_F(DefineObjectTest, RepeatedClassifyIsIndependent)
{
    for (int i = 0; i < 5; ++i)
    {
        Result *res = classify_testobj(TEXT_PAGE, ctx, doc);
        ASSERT_EQ(res->type, Result::OK);

        res->deleter(res->payload);
        delete res;
    }

    EXPECT_EQ(g_objects_alive, 0);
}
