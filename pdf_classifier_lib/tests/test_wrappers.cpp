#include "../wrappers.hpp"

#include <gtest/gtest.h>

#include <filesystem>
#include <memory>
#include <type_traits>
#include <vector>

// ---------------------------------------------------------------------------
// Part 1 - ownership semantics, using a fake resource type.
//
// FzOwned<T, Drop> never inspects T, so a plain struct with a counting drop
// function exercises every ownership path deterministically and without MuPDF.
// These are the tests that pin the behaviour we actually care about: exactly
// one drop per resource, never zero, never two.
// ---------------------------------------------------------------------------

namespace
{
    struct Tracked
    {
        int id;
    };

    int g_drop_count = 0;
    fz_context *g_last_drop_ctx = nullptr;
    int g_last_drop_id = -1;

    void track_drop(fz_context *ctx, Tracked *t)
    {
        ++g_drop_count;
        g_last_drop_ctx = ctx;
        g_last_drop_id = t->id;
        delete t;
    }

    using TrackedOwned = FzOwned<Tracked, track_drop>;

    // Stand-in for a real context. Never dereferenced by these tests - FzOwned
    // only ever passes it through to Drop.
    fz_context *fake_ctx()
    {
        return reinterpret_cast<fz_context *>(0xF00D);
    }

    TrackedOwned make_tracked(int id)
    {
        return TrackedOwned{fake_ctx(), new Tracked{id}};
    }
}

class OwnedFixture : public ::testing::Test
{
protected:
    void SetUp() override
    {
        g_drop_count = 0;
        g_last_drop_ctx = nullptr;
        g_last_drop_id = -1;
    }
};

// The type must not be copyable - a copy would double-free. It must be movable,
// or it cannot be returned from make() or stored in a container.
TEST_F(OwnedFixture, TypeTraits)
{
    static_assert(!std::is_copy_constructible_v<TrackedOwned>);
    static_assert(!std::is_copy_assignable_v<TrackedOwned>);
    static_assert(std::is_move_constructible_v<TrackedOwned>);
    static_assert(std::is_move_assignable_v<TrackedOwned>);
    static_assert(std::is_same_v<TrackedOwned::element_type, Tracked>);
    SUCCEED();
}

TEST_F(OwnedFixture, DestructorDropsExactlyOnce)
{
    {
        TrackedOwned owned = make_tracked(7);
        EXPECT_EQ(g_drop_count, 0) << "must not drop while still owned";
    }

    EXPECT_EQ(g_drop_count, 1);
    EXPECT_EQ(g_last_drop_id, 7);
}

// Drop takes the context the resource was created with. Passing the wrong one
// is the cross-thread free this whole design exists to prevent, so pin it.
TEST_F(OwnedFixture, DropReceivesOriginatingContext)
{
    {
        TrackedOwned owned = make_tracked(1);
    }

    EXPECT_EQ(g_last_drop_ctx, fake_ctx());
}

TEST_F(OwnedFixture, DefaultConstructedDropsNothing)
{
    {
        TrackedOwned owned;
        EXPECT_FALSE(static_cast<bool>(owned));
        EXPECT_EQ(owned.get(), nullptr);
    }

    EXPECT_EQ(g_drop_count, 0);
}

TEST_F(OwnedFixture, MoveConstructionTransfersOwnership)
{
    {
        TrackedOwned src = make_tracked(3);
        Tracked *raw = src.get();

        TrackedOwned dst = std::move(src);

        EXPECT_EQ(dst.get(), raw);
        EXPECT_EQ(src.get(), nullptr) << "moved-from must be emptied or it double-drops";
        EXPECT_FALSE(static_cast<bool>(src));
        EXPECT_EQ(g_drop_count, 0);
    }

    EXPECT_EQ(g_drop_count, 1) << "one resource, one drop - not two";
}

TEST_F(OwnedFixture, MoveAssignmentDropsPreviousResource)
{
    {
        TrackedOwned dst = make_tracked(1);
        TrackedOwned src = make_tracked(2);

        dst = std::move(src);

        EXPECT_EQ(g_drop_count, 1) << "the overwritten resource must be dropped";
        EXPECT_EQ(g_last_drop_id, 1);
        EXPECT_EQ(src.get(), nullptr);
    }

    EXPECT_EQ(g_drop_count, 2);
    EXPECT_EQ(g_last_drop_id, 2);
}

TEST_F(OwnedFixture, SelfMoveAssignmentIsSafe)
{
    {
        TrackedOwned owned = make_tracked(5);
        TrackedOwned &alias = owned;

        owned = std::move(alias);

        EXPECT_EQ(g_drop_count, 0) << "self-assignment must not destroy the resource";
        ASSERT_NE(owned.get(), nullptr);
        EXPECT_EQ(owned.get()->id, 5);
    }

    EXPECT_EQ(g_drop_count, 1);
}

TEST_F(OwnedFixture, ReleaseSuppressesDrop)
{
    Tracked *raw = nullptr;

    {
        TrackedOwned owned = make_tracked(9);
        raw = owned.release();

        EXPECT_EQ(owned.get(), nullptr);
    }

    EXPECT_EQ(g_drop_count, 0) << "release() hands ownership to the caller";

    ASSERT_NE(raw, nullptr);
    EXPECT_EQ(raw->id, 9);
    delete raw;
}

TEST_F(OwnedFixture, ResetIsIdempotent)
{
    TrackedOwned owned = make_tracked(4);

    owned.reset();
    EXPECT_EQ(g_drop_count, 1);

    owned.reset();
    owned.reset();
    EXPECT_EQ(g_drop_count, 1) << "reset() on an empty handle must be a no-op";
}

TEST_F(OwnedFixture, ResetThenDestructorDropsOnce)
{
    {
        TrackedOwned owned = make_tracked(6);
        owned.reset();
    }

    EXPECT_EQ(g_drop_count, 1);
}

// Regression test for object slicing.
//
// Storing FzOwned by value in a vector<FzOwnedResource> silently discards the
// derived part, so ~FzOwnedResource runs instead of ~FzOwned and nothing is
// ever dropped. Polymorphic storage requires indirection.
TEST_F(OwnedFixture, PolymorphicStorageStillDrops)
{
    {
        std::vector<std::unique_ptr<FzOwnedResource>> resources;

        resources.push_back(std::make_unique<TrackedOwned>(make_tracked(11)));
        resources.push_back(std::make_unique<TrackedOwned>(make_tracked(12)));

        EXPECT_EQ(g_drop_count, 0);
    }

    EXPECT_EQ(g_drop_count, 2) << "dropping through the erased base requires a virtual destructor";
}

TEST_F(OwnedFixture, ClearingPolymorphicStorageDrops)
{
    std::vector<std::unique_ptr<FzOwnedResource>> resources;
    resources.push_back(std::make_unique<TrackedOwned>(make_tracked(21)));

    resources.clear();

    EXPECT_EQ(g_drop_count, 1);
}

// Vector growth moves the unique_ptrs, not the FzOwneds, so the raw pointers
// handed out by allocate() stay valid across reallocation.
TEST_F(OwnedFixture, RawPointersSurviveVectorReallocation)
{
    std::vector<std::unique_ptr<FzOwnedResource>> resources;

    auto first = std::make_unique<TrackedOwned>(make_tracked(31));
    Tracked *raw = first->get();
    resources.push_back(std::move(first));

    for (int i = 0; i < 64; ++i)
        resources.push_back(std::make_unique<TrackedOwned>(make_tracked(100 + i)));

    EXPECT_EQ(raw->id, 31);
    EXPECT_EQ(g_drop_count, 0);
}

// ---------------------------------------------------------------------------
// Part 2 - fz_call against real MuPDF.
//
// These need the test document. They cover the setjmp/longjmp boundary, which
// is the part that cannot be reasoned about from the source alone.
// ---------------------------------------------------------------------------

class FzCallFixture : public ::testing::Test
{
protected:
    fz_context *ctx = nullptr;
    fz_document *doc = nullptr;

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

TEST_F(FzCallFixture, ReturnsValueOnSuccess)
{
    fz_page *page = fz_call(ctx, fz_load_page, doc, 0);

    ASSERT_NE(page, nullptr);
    fz_drop_page(ctx, page);
}

TEST_F(FzCallFixture, ThrowsFzErrorOnFailure)
{
    EXPECT_THROW(fz_call(ctx, fz_load_page, doc, 99999999), FzError);
}

TEST_F(FzCallFixture, FzErrorCarriesCodeAndMessage)
{
    try
    {
        fz_call(ctx, fz_load_page, doc, 99999999);
        FAIL() << "expected FzError";
    }
    catch (const FzError &e)
    {
        EXPECT_NE(e.code, 0);
        EXPECT_STRNE(e.what(), "");
    }
}

// The critical one. fz_try is a setjmp; throwing out of fz_catch is only safe
// if the exception frame was already popped. If it wasn't, the context is
// corrupt and the *next* call misbehaves - so a failure here shows up as the
// successful call below breaking, not as the throwing one.
TEST_F(FzCallFixture, ContextRemainsUsableAfterCaughtError)
{
    for (int i = 0; i < 3; ++i)
        EXPECT_THROW(fz_call(ctx, fz_load_page, doc, 99999999), FzError);

    fz_page *page = fz_call(ctx, fz_load_page, doc, 0);

    ASSERT_NE(page, nullptr) << "exception stack left unbalanced by the throw in fz_catch";
    fz_drop_page(ctx, page);
}

TEST_F(FzCallFixture, HandlesVoidReturningFunctions)
{
    fz_call(ctx, fz_set_aa_level, 0);
    EXPECT_EQ(fz_aa_level(ctx), 0);

    fz_call(ctx, fz_set_aa_level, 8);
    EXPECT_EQ(fz_aa_level(ctx), 8);
}

// ---------------------------------------------------------------------------
// Part 3 - FzOwned::make against real MuPDF.
// ---------------------------------------------------------------------------

class MakeFixture : public FzCallFixture
{
};

TEST_F(MakeFixture, MakeProducesOwnedPage)
{
    {
        FzPage page = FzPage::make(ctx, fz_load_page, doc, 0);
        EXPECT_TRUE(static_cast<bool>(page));
        ASSERT_NE(page.get(), nullptr);
    }

    SUCCEED() << "page dropped without crashing";
}

TEST_F(MakeFixture, MakePropagatesFzError)
{
    EXPECT_THROW(FzPage::make(ctx, fz_load_page, doc, 99999999), FzError);
}

TEST_F(MakeFixture, MakeChainsFromAnotherOwnedResource)
{
    FzPage page = FzPage::make(ctx, fz_load_page, doc, 0);

    fz_stext_options opts = {};
    FzSTextPage stext = FzSTextPage::make(ctx, fz_new_stext_page_from_page, page.get(), &opts);

    EXPECT_TRUE(static_cast<bool>(stext));
}

TEST_F(MakeFixture, MovedOwnedDropsOnlyOnce)
{
    FzPage first = FzPage::make(ctx, fz_load_page, doc, 0);
    fz_page *raw = first.get();

    FzPage second = std::move(first);

    EXPECT_EQ(second.get(), raw);
    EXPECT_EQ(first.get(), nullptr);
}

// Not testable at runtime - static_assert failures are compile errors, so
// these are documented rather than exercised. Uncomment one to confirm the
// diagnostic is the intended message rather than a template error wall:
//h
//   FzPage::make(ctx, fz_load_page, doc);              // too few arguments
//   FzPage::make(ctx, fz_load_page, ctx, doc, 0);      // ctx passed twice
//   FzPage::make(ctx, fz_new_pixmap_from_page, ...);   // returns the wrong type
