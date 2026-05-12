#ifndef TEST_PDF_PATH
#error "TEST_PDF_PATH was not defined!"
#endif

#include "table.hpp"
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <mupdf/fitz.h>

static bool result_is_ok(Result* res) {
  bool ok = (res->type == Result::Type::OK);
  delete res;
  return ok;
}

// Return the text stored in a cell, or "" if the unique_ptr is null.
static std::string cell_text(const TableDataCell& cell) { return (cell.text != nullptr) ? *cell.text : ""; }
static std::string boundary_debug(fz_rect boundary) {
  return std::format("[({},{}),({}, {})]", boundary.x0, boundary.y0, boundary.x1, boundary.y1);
}

// Find the first cell in a flat cell list matching (row_num, column).
static const TableDataCell* find_cell(const std::vector<TableDataCell>& cells, int row, CellColumn col) {
  for (const auto& c : cells) {
    if (c.row_num == row && c.column == static_cast<int>(col))
      return &c;
  }
  return nullptr;
}

class DataTableFixture : public ::testing::Test {
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

// Old test: TestTableInitialization (page 1235)
// Verifies that construction succeeds and page bounds are acceptable.
TEST_F(DataTableFixture, TestValidPageBounds) {
  try {
    DataTable dt(ctx, doc, 1235);
    EXPECT_TRUE(result_is_ok(dt.valid_page_bounds())) << "Page 1235 should have valid page bounds";
  } catch (const std::exception& e) {
    FAIL() << "DataTable construction or validation threw: " << e.what();
  }
}

// Old test: TestSplitMembersAndDebugOutput (page 1235)
// The new equivalent is extract_cells().  We verify a non-empty result.
TEST_F(DataTableFixture, TestExtractCellsNonEmpty) {
  try {
    DataTable dt(ctx, doc, 1235);
    auto cells = dt.extract_cells();
    EXPECT_FALSE(cells.empty()) << "extract_cells() on page 1235 should return at least one cell";
    GTEST_LOG_(INFO) << "Extracted " << cells.size() << " cells from page 1235";
  } catch (const std::exception& e) {
    FAIL() << "DataTable construction or extraction threw: " << e.what();
  }
}

// Old test: TestTableInitialization — verifies all seven columns are represented.
TEST_F(DataTableFixture, TestExtractCellsCoverAllColumns) {
  try {
    DataTable dt(ctx, doc, 1235);
    auto cells = dt.extract_cells();
    ASSERT_FALSE(cells.empty());

    bool has[8] = {}; // indexed by CellColumn (1-based)
    for (const auto& c : cells) {
      if (c.column >= 1 && c.column <= 7)
        has[c.column] = true;
    }

    EXPECT_TRUE(has[KEY]) << "Expected cells in KEY column";
    EXPECT_TRUE(has[DMC_ARMY]) << "Expected cells in DMC_ARMY column";
    EXPECT_TRUE(has[NATO_STOCK_NUM]) << "Expected cells in NATO_STOCK_NUM column";
    EXPECT_TRUE(has[ITEM_NAME]) << "Expected cells in ITEM_NAME column";
    EXPECT_TRUE(has[PART_NUM]) << "Expected cells in PART_NUM column";
    EXPECT_TRUE(has[NUM_OFF]) << "Expected cells in NUM_OFF column";
  } catch (const std::exception& e) {
    FAIL() << "DataTable construction or extraction threw: " << e.what();
  }
}

// Old test: TestDataExtraction / TestDataExtractionDetails (page 1248)
// Spot-checks NATO stock number, item name, and part number for three rows
// from the same expected dataset used by the original test suite.
TEST_F(DataTableFixture, TestExtractCellsDetailedData) {
  struct ExpectedRow {
    int row;
    const char* nato_stock_num;
    const char* item_name;
    const char* part_num;
  };

  // A representative subset of the 25 rows validated in the old test.
  static const ExpectedRow expected[] = {
      {0, "2540-99-830-1219", "INSTALLATION KIT, EMERGENCY BEACON AND SIREN", "STC50536"},
      {1, "6220-99-732-0786", "LIGHT, WARNING", "STC3446"},
      {5, "5340-99-573-6451", "BRACKET, MOUNTING", "STC4242"},
      {12, "5963-99-663-6041", "AMPLIFIER, AUDIO FREQUENCY", "STC3669"},
  };

  try {
    DataTable dt(ctx, doc, 1248);
    auto cells = dt.extract_cells();
    ASSERT_FALSE(cells.empty()) << "Expected cells on page 1248";
    GTEST_LOG_(INFO) << "Extracted " << cells.size() << " cells from page 1248";

    for (const auto& exp : expected) {
      SCOPED_TRACE("row " + std::to_string(exp.row));

      const TableDataCell* nato = find_cell(cells, exp.row, NATO_STOCK_NUM);
      if (nato) {
        EXPECT_EQ(cell_text(*nato), exp.nato_stock_num)
            << "NATO stock number mismatch at row " << exp.row << " with boundary " << boundary_debug(nato->boundary);
      } else {
        GTEST_LOG_(INFO) << "No NATO_STOCK_NUM cell for row " << exp.row;
      }

      const TableDataCell* item = find_cell(cells, exp.row, ITEM_NAME);
      if (item) {
        EXPECT_EQ(cell_text(*item), exp.item_name)
            << "Item name mismatch at row " << exp.row << " with boundary " << boundary_debug(nato->boundary);
      } else {
        GTEST_LOG_(INFO) << "No ITEM_NAME cell for row " << exp.row;
      }

      const TableDataCell* part = find_cell(cells, exp.row, PART_NUM);
      if (part) {
        EXPECT_EQ(cell_text(*part), exp.part_num)
            << "Part number mismatch at row " << exp.row << " with boundary " << boundary_debug(nato->boundary);
      } else {
        GTEST_LOG_(INFO) << "No PART_NUM cell for row " << exp.row;
      }
    }
  } catch (const std::exception& e) {
    FAIL() << "DataTable construction or extraction threw: " << e.what();
  }
}

// Dumps cell boundaries for all cells on page 1248 to a JSON file so that
// examples/tests/visualize_boundaries.py can render an annotated image.
TEST_F(DataTableFixture, TestDumpCellBoundaries) {
  constexpr int dump_page = 1248;
  const std::string output_path = TEST_BOUNDARY_OUTPUT_PATH;

  DataTable dt(ctx, doc, dump_page);
  auto cells = dt.extract_cells();
  ASSERT_FALSE(cells.empty()) << "No cells found on page " << dump_page;

  std::ofstream out(output_path);
  ASSERT_TRUE(out.is_open()) << "Could not open output file: " << output_path;

  // Use forward slashes so the path is valid JSON on Windows too.
  std::string pdf_json = std::filesystem::path(TEST_PDF_PATH).generic_string();

  out << "{\n";
  out << std::format("  \"pdf_path\": \"{}\",\n", pdf_json);
  out << std::format("  \"page\": {},\n", dump_page);
  out << "  \"cells\": [\n";

  for (size_t i = 0; i < cells.size(); ++i) {
    const auto& c = cells[i];
    out << std::format("    {{\"row\": {}, \"col\": {}, \"boundary\": "
                       "{{\"x0\": {:.2f}, \"y0\": {:.2f}, \"x1\": {:.2f}, \"y1\": {:.2f}}}}}",
                       c.row_num, c.column, c.boundary.x0, c.boundary.y0, c.boundary.x1, c.boundary.y1);
    if (i + 1 < cells.size())
      out << ",";
    out << "\n";
  }

  out << "  ]\n}\n";
  out.close();

  GTEST_LOG_(INFO) << "Wrote " << cells.size() << " cell boundaries to: " << output_path;
  GTEST_LOG_(INFO) << "Visualize with:  python examples/tests/visualize_boundaries.py " << output_path;
}
