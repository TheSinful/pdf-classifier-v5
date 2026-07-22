#pragma once

#ifndef COLORDOC_TEST_PDF_PATH
#error "COLORDOC_TEST_PDF_PATH was not defined!"
#endif

#ifndef COLORDOC_TEST_GROUNDTRUTH_PATH
#error "COLORDOC_TEST_GROUNDTRUTH_PATH was not defined!"
#endif

#include "palette.hpp"
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <mupdf/fitz.h>
#include <nlohmann/json.hpp>
#include <shared/result.h>
#include <string>
#include <vector>

/// `Result` owns neither its payload nor its deleter's work - the FFI layer
/// runs the deleter after reading the payload out. Tests have to do the same or
/// they leak a `ColorPage` per classify call.
inline void release_payload(Result* res) {
  if (res && res->type == Result::Type::OK && res->payload && res->deleter) {
    res->deleter(res->payload);
  }
  delete res;
}

/// Opens the generated fixture document alongside its ground truth, so every
/// test can ask "what is page n really?" without hard-coding page numbers - the
/// fixture is regenerated (deterministically, from a fixed seed) by
/// `generate_doc.py`, so hard-coded indices would rot immediately.
class ColordocFixture : public ::testing::Test {
protected:
  fz_context* ctx = nullptr;
  fz_document* doc = nullptr;

  /// pages[n] is the true class of page n; `blank_class` for the blank pages.
  std::vector<std::string> pages;
  std::string blank_class;

  void SetUp() override {
    std::filesystem::path pdf_path = COLORDOC_TEST_PDF_PATH;
    std::filesystem::path truth_path = COLORDOC_TEST_GROUNDTRUTH_PATH;

    if (!std::filesystem::exists(pdf_path) || !std::filesystem::exists(truth_path)) {
      GTEST_SKIP() << "colordoc fixture not generated; run: python generate_doc.py --seed 7 "
                      "--max-pages 60 --out test_data/fixture.pdf";
    }

    std::ifstream truth_file(truth_path);
    ASSERT_TRUE(truth_file.is_open()) << "could not open ground truth at " << truth_path;

    nlohmann::json truth = nlohmann::json::parse(truth_file);
    pages = truth.at("pages").get<std::vector<std::string>>();
    blank_class = truth.at("blank_class").get<std::string>();
    ASSERT_FALSE(pages.empty()) << "ground truth lists no pages";

    ctx = fz_new_context(NULL, NULL, FZ_STORE_UNLIMITED);
    ASSERT_NE(ctx, nullptr) << "Failed to create MuPDF context";
    fz_try(ctx) {
      fz_register_document_handlers(ctx);
      doc = fz_open_document(ctx, pdf_path.string().c_str());
      if (!doc)
        fz_throw(ctx, FZ_ERROR_GENERIC, "Failed to open document");
    }
    fz_catch(ctx) { FAIL() << "MuPDF error: " << fz_caught_message(ctx); }

    ASSERT_EQ(static_cast<size_t>(fz_count_pages(ctx, doc)), pages.size())
        << "fixture PDF and ground truth disagree on page count";
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

  /// Consumes a Result (payload included), surfacing its failure reason.
  static ::testing::AssertionResult is_ok(Result* res) {
    if (!res) {
      return ::testing::AssertionFailure() << "classifier returned a null Result";
    }

    const bool ok = res->type == Result::Type::OK;
    const std::string reason = ok ? "" : res->fail_rsn;
    release_payload(res);

    if (ok) {
      return ::testing::AssertionSuccess();
    }
    return ::testing::AssertionFailure() << reason;
  }

  /// Consumes a Result expected to fail, surfacing the payload it wrongly built.
  static ::testing::AssertionResult is_fail(Result* res) {
    if (!res) {
      return ::testing::AssertionFailure() << "classifier returned a null Result";
    }

    const bool ok = res->type == Result::Type::OK;
    release_payload(res);

    if (!ok) {
      return ::testing::AssertionSuccess();
    }
    return ::testing::AssertionFailure() << "expected a failure but the classifier accepted the page";
  }

  /// Consumes an extraction Result, copying its JSON payload out before the
  /// payload's deleter runs.
  static bool read_json_payload(Result* res, std::string& out, std::string& fail_reason) {
    if (!res) {
      fail_reason = "extract returned a null Result";
      return false;
    }

    if (res->type != Result::Type::OK) {
      fail_reason = res->fail_rsn;
      delete res;
      return false;
    }

    if (!res->payload) {
      fail_reason = "extract succeeded with a null payload";
      delete res;
      return false;
    }

    out = *static_cast<std::string*>(res->payload);
    release_payload(res);
    return true;
  }

  static Rgb expected_color(const std::string& class_name) {
    if (class_name == "section")
      return SECTION_COLOR;
    if (class_name == "subsection")
      return SUBSECTION_COLOR;
    if (class_name == "figure")
      return FIGURE_COLOR;
    if (class_name == "caption")
      return CAPTION_COLOR;
    return BLANK_COLOR;
  }
};
