#include "../result.hpp"
#include "../util.hpp"

#include <gtest/gtest.h>

#include <type_traits>

// ---------------------------------------------------------------------------
// ClassificationResult
// ---------------------------------------------------------------------------

// These are returned by value from user overrides, so they must behave like
// values. The previous design inherited the ABI Result, which is copy- and
// move-deleted and could not.
TEST(ClassificationResultTraits, IsAUsableValueType)
{
    static_assert(std::is_copy_constructible_v<ClassificationResult>);
    static_assert(std::is_move_constructible_v<ClassificationResult>);
    static_assert(std::is_destructible_v<ClassificationResult>);
    SUCCEED();
}

TEST(ClassificationResultTest, OkReportsSuccess)
{
    const ClassificationResult r = ClassificationResult::ok();
    EXPECT_TRUE(r.is_ok());
}

TEST(ClassificationResultTest, FailCarriesReason)
{
    const ClassificationResult r = ClassificationResult::fail("doesn't contain an image");

    EXPECT_FALSE(r.is_ok());
    EXPECT_EQ(r.failure(), "doesn't contain an image");
}

TEST(ClassificationResultTest, SurvivesCopyAndMove)
{
    ClassificationResult original = ClassificationResult::fail("boom");

    const ClassificationResult copied = original;
    EXPECT_FALSE(copied.is_ok());
    EXPECT_EQ(copied.failure(), "boom");

    const ClassificationResult moved = std::move(original);
    EXPECT_FALSE(moved.is_ok());
    EXPECT_EQ(moved.failure(), "boom");
}

// ---------------------------------------------------------------------------
// ExtractionResult
// ---------------------------------------------------------------------------

TEST(ExtractionResultTraits, IsAUsableValueType)
{
    static_assert(std::is_copy_constructible_v<ExtractionResult>);
    static_assert(std::is_move_constructible_v<ExtractionResult>);
    static_assert(std::is_destructible_v<ExtractionResult>);
    SUCCEED();
}

TEST(ExtractionResultTest, OkCarriesData)
{
    const ExtractionResult r = ExtractionResult::ok(std::string("payload"));

    EXPECT_TRUE(r.is_ok());
    EXPECT_EQ(r.data(), "payload");
}

TEST(ExtractionResultTest, OkAcceptsAnythingIntoString)
{
    EXPECT_EQ(ExtractionResult::ok("literal").data(), "literal");
    EXPECT_EQ(ExtractionResult::ok(std::string_view("view")).data(), "view");

    const nlohmann::json j = {{"fig_num", 3}, {"caption", "a caption"}};
    EXPECT_EQ(ExtractionResult::ok(j).data(), j.dump());
}

TEST(ExtractionResultTest, FailCarriesReason)
{
    const ExtractionResult r = ExtractionResult::fail("nothing to extract");

    EXPECT_FALSE(r.is_ok());
    EXPECT_EQ(r.failure(), "nothing to extract");
}

// The macro moves the payload out rather than copying it - DataTable's JSON is
// every cell on the page.
TEST(ExtractionResultTest, TakeDataMovesThePayloadOut)
{
    ExtractionResult r = ExtractionResult::ok(std::string("a reasonably long payload"));

    const std::string taken = std::move(r).take_data();

    EXPECT_EQ(taken, "a reasonably long payload");
}

// take_data() is &&-qualified, so a named object cannot be gutted by accident -
// only an explicit std::move reaches it. Uncommenting the first line below
// should fail to compile:
//
//   ExtractionResult named = ExtractionResult::ok("x");
//   named.take_data();   // error: cannot convert 'this' to ExtractionResult&&
TEST(ExtractionResultTest, TakeDataIsReachableOnlyThroughAnRvalue)
{
    EXPECT_EQ(ExtractionResult::ok("temporary").take_data(), "temporary");

    ExtractionResult named = ExtractionResult::ok("named");
    EXPECT_EQ(std::move(named).take_data(), "named");
}

// ---------------------------------------------------------------------------
// UNWRAP
// ---------------------------------------------------------------------------

namespace
{
    int g_calls = 0;

    ClassificationResult failing_check()
    {
        ++g_calls;
        return ClassificationResult::fail("check failed");
    }

    ClassificationResult passing_check()
    {
        ++g_calls;
        return ClassificationResult::ok();
    }

    ClassificationResult unwraps_a_failure()
    {
        UNWRAP(failing_check());
        return ClassificationResult::ok();
    }

    // The sentinel proves control reached the line *after* the UNWRAP.
    ClassificationResult unwraps_a_pass()
    {
        UNWRAP(passing_check());
        return ClassificationResult::fail("reached the end");
    }

    ClassificationResult unwraps_several()
    {
        UNWRAP(passing_check());
        UNWRAP(passing_check());
        UNWRAP(failing_check());
        return ClassificationResult::ok();
    }
}

class UnwrapTest : public ::testing::Test
{
protected:
    void SetUp() override { g_calls = 0; }
};

TEST_F(UnwrapTest, ReturnsEarlyOnFailure)
{
    const ClassificationResult r = unwraps_a_failure();

    EXPECT_FALSE(r.is_ok());
    EXPECT_EQ(r.failure(), "check failed");
}

TEST_F(UnwrapTest, FallsThroughOnSuccess)
{
    const ClassificationResult r = unwraps_a_pass();

    EXPECT_FALSE(r.is_ok());
    EXPECT_EQ(r.failure(), "reached the end")
        << "UNWRAP must not return early when the check passes";
    EXPECT_EQ(g_calls, 1);
}

// Regression test: the first version of the macro named its argument twice, so
// the checked expression ran a second time on the failure path - re-running
// regexes and re-assigning members.
TEST_F(UnwrapTest, EvaluatesItsArgumentExactlyOnce)
{
    unwraps_a_failure();
    EXPECT_EQ(g_calls, 1) << "UNWRAP must not re-evaluate its expression";
}

TEST_F(UnwrapTest, ShortCircuitsRemainingChecks)
{
    const ClassificationResult r = unwraps_several();

    EXPECT_FALSE(r.is_ok());
    EXPECT_EQ(g_calls, 3) << "checks after the failing one must not run";
}

// Multiple UNWRAPs in one scope must not collide on the internal name.
TEST_F(UnwrapTest, NestsWithinOneScope)
{
    EXPECT_FALSE(unwraps_several().is_ok());
}
