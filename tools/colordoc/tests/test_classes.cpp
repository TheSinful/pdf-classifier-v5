#include "colordoc_fixture.hpp"
#include "classes.hpp"

#include <set>

using nlohmann::json;

namespace {

struct ClassEntry {
  const char* name;
  Result* (*classify)(uint32_t, fz_context*, fz_document*);
  Result* (*extract)(uint32_t, fz_context*, fz_document*, void*);
};

constexpr ClassEntry CLASSES[] = {
    {"section", classify_section, extract_section},
    {"subsection", classify_subsection, extract_subsection},
    {"figure", classify_figure, extract_figure},
    {"caption", classify_caption, extract_caption},
};

} // namespace

using ClassesTest = ColordocFixture;

// The fixture is randomly generated; if a seed ever produced a document missing
// a class, the sweep below would pass vacuously for it.
TEST_F(ClassesTest, FixtureExercisesEveryClass) {
  const std::set<std::string> present(pages.begin(), pages.end());

  for (const ClassEntry& entry : CLASSES) {
    EXPECT_TRUE(present.count(entry.name) > 0) << "fixture contains no " << entry.name << " page";
  }
  EXPECT_TRUE(present.count(blank_class) > 0) << "fixture contains no blank page";
}

// The central guarantee: on every page, exactly one classifier accepts, and it
// is the right one. Everything the engine does downstream assumes this.
TEST_F(ClassesTest, EveryPageIsAcceptedByExactlyItsOwnClass) {
  for (size_t page = 0; page < pages.size(); ++page) {
    const std::string& truth = pages[page];

    for (const ClassEntry& entry : CLASSES) {
      Result* res = entry.classify(static_cast<uint32_t>(page), ctx, doc);

      if (truth == entry.name) {
        EXPECT_TRUE(is_ok(res)) << "page " << page << " is a " << truth << " but classify_" << entry.name
                                << " rejected it";
      } else {
        EXPECT_TRUE(is_fail(res)) << "page " << page << " is a " << truth << " but classify_" << entry.name
                                  << " also accepted it";
      }
    }
  }
}

// Blank pages back the BlankAfter override: the engine only records them as
// `unknown` because no classifier will take them.
TEST_F(ClassesTest, BlankPagesAreRejectedByEveryClass) {
  size_t checked = 0;

  for (size_t page = 0; page < pages.size(); ++page) {
    if (pages[page] != blank_class) {
      continue;
    }

    ++checked;
    for (const ClassEntry& entry : CLASSES) {
      EXPECT_TRUE(is_fail(entry.classify(static_cast<uint32_t>(page), ctx, doc)))
          << "blank page " << page << " was accepted by classify_" << entry.name;
    }
  }

  EXPECT_GT(checked, 0u) << "fixture had no blank pages to check";
}

// A classify is re-run whenever the engine probes a page during deferral, so an
// unstable verdict would make recovery non-deterministic.
TEST_F(ClassesTest, ClassificationIsRepeatable) {
  for (size_t page = 0; page < pages.size(); ++page) {
    for (const ClassEntry& entry : CLASSES) {
      const bool first = is_ok(entry.classify(static_cast<uint32_t>(page), ctx, doc));
      const bool second = is_ok(entry.classify(static_cast<uint32_t>(page), ctx, doc));

      EXPECT_EQ(first, second) << "classify_" << entry.name << " gave different verdicts for page " << page;
    }
  }
}

// Extraction consumes the `Shared` payload the classify produced, and its JSON
// is what reaches the Python frontend - so it has to describe the page it was
// actually run on.
TEST_F(ClassesTest, ExtractionDescribesThePageItRanOn) {
  for (size_t page = 0; page < pages.size(); ++page) {
    const std::string& truth = pages[page];
    if (truth == blank_class) {
      continue;
    }

    const ClassEntry* entry = nullptr;
    for (const ClassEntry& candidate : CLASSES) {
      if (truth == candidate.name) {
        entry = &candidate;
      }
    }
    ASSERT_NE(entry, nullptr) << "ground truth names an unknown class: " << truth;

    Result* classified = entry->classify(static_cast<uint32_t>(page), ctx, doc);
    ASSERT_NE(classified, nullptr);
    ASSERT_EQ(classified->type, Result::Type::OK) << "page " << page << ": " << classified->fail_rsn;

    void* shared = classified->payload;
    void (*shared_deleter)(void*) = classified->deleter;
    ASSERT_NE(shared, nullptr) << "classify_" << entry->name << " succeeded without a shared payload";
    delete classified; // the payload outlives the Result; it is freed below.

    std::string payload;
    std::string reason;
    const bool extracted = read_json_payload(entry->extract(static_cast<uint32_t>(page), ctx, doc, shared),
                                             payload, reason);

    if (shared_deleter) {
      shared_deleter(shared);
    }

    ASSERT_TRUE(extracted) << "extract_" << entry->name << " failed on page " << page << ": " << reason;

    const json parsed = json::parse(payload);
    EXPECT_EQ(parsed.at("class").get<std::string>(), truth);
    EXPECT_EQ(parsed.at("page").get<size_t>(), page);

    const Rgb expected = expected_color(truth);
    const std::vector<int> rgb = parsed.at("rgb").get<std::vector<int>>();
    ASSERT_EQ(rgb.size(), 3u);
    EXPECT_EQ(rgb[0], static_cast<int>(expected.r));
    EXPECT_EQ(rgb[1], static_cast<int>(expected.g));
    EXPECT_EQ(rgb[2], static_cast<int>(expected.b));
  }
}

// The engine hands extract whatever the classify stashed; a null there means a
// wiring bug, and must not be dereferenced.
TEST_F(ClassesTest, ExtractionRejectsMissingSharedPayload) {
  for (const ClassEntry& entry : CLASSES) {
    EXPECT_TRUE(is_fail(entry.extract(0, ctx, doc, nullptr)))
        << "extract_" << entry.name << " accepted a null shared payload";
  }
}

// Out-of-range pages reach classify during a deferral scan that runs off the end
// of the document; they must fail rather than throw across the FFI boundary.
TEST_F(ClassesTest, OutOfRangePageIsRejectedNotThrown) {
  const uint32_t past_end = static_cast<uint32_t>(pages.size() + 16);

  for (const ClassEntry& entry : CLASSES) {
    ASSERT_NO_THROW({
      EXPECT_TRUE(is_fail(entry.classify(past_end, ctx, doc)))
          << "classify_" << entry.name << " accepted a page past the end of the document";
    });
  }
}
