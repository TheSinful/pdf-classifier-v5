#pragma once

#include "object.hpp"
#include "util.hpp"
#include <memory>
#include <mupdf/fitz.h>
#include <shared/result.h>
#include <variant>
#include <vector>

/*
    Since DataTables are the heavy data pages that are going to be
    significantly more expensive than diagrams or subchapters.
    Datatable classification holds little value, relying heavily on the fact
    that they ONLY show after diagrams, no matter what.
*/

using CellVerticalBoundaryList = std::vector<std::pair<float, float>>;

struct TableDataCell {
  fz_rect boundary;
  int row_num;
  int column;
  std::unique_ptr<std::string> text;
};

struct TableDataKeyCell : public TableDataCell {
  bool included;
  int key;
};

struct TableDataCountCell : public TableDataCell {
  int count;
};

NLOHMANN_DEFINE_TYPE_NON_INTRUSIVE(fz_rect, x0, x1, y0, y1)

using TableCellKind = std::variant<TableDataCell, TableDataKeyCell, TableDataCountCell>;

inline void to_json(nlohmann::json& j, const TableCellKind& c) {
  j = {};

  std::visit(
      [&](auto&& v) {
        using T = std::decay_t<decltype(v)>;
        if constexpr (std::is_same_v<T, TableDataKeyCell>) {
          j["boundary"] = v.boundary;
          j["row_num"] = v.row_num;
          j["column"] = v.column;
          j["text"] = v.text ? *v.text : "";
          j["included"] = v.included;
          j["key"] = v.key;
        } else if constexpr (std::is_same_v<T, TableDataCountCell>) {
          j["boundary"] = v.boundary;
          j["row_num"] = v.row_num;
          j["column"] = v.column;
          j["text"] = v.text ? *v.text : "";
          j["count"] = v.count;
        } else {
          j["boundary"] = v.boundary;
          j["row_num"] = v.row_num;
          j["column"] = v.column;
          j["text"] = v.text ? *v.text : "";
        }
      },
      c);
}

enum CellColumn { KEY = 1, DMC_ARMY = 2, NATO_STOCK_NUM = 3, ITEM_NAME = 4, PART_NUM = 5, NUM_OFF = 6, ANNOTATION = 7 };

class DataTable : public Object<true, true> {
public:
  DataTable(fz_context* ctx, fz_document* doc, uint32_t page);
  ~DataTable();

  Result* valid_page_bounds();
  std::vector<TableCellKind> extract_cells();

private:
  void calc_page_scope();
  void calc_column_boundaries();
  std::vector<int> calc_column_seperators(fz_rect scan_bounds);
  CellVerticalBoundaryList calc_cell_vertical_boundaires(int start_y, int end_y, int start_x, int end_x);
  void construct_cells(const CellVerticalBoundaryList& cell_vertical_boundaries);
  TableDataKeyCell parse_key_column(const std::string& text, TableDataCell cell);
  TableDataCountCell parse_count_column(const std::string& text, TableDataCell cell);
  bool is_vertical_line_filled(int x, int start_y, int end_y);
  bool is_white_pixel(unsigned char* pixel);
  bool row_has_content(int y, int start_x, int end_x);
  std::string extract_text_from_cell(const TableDataCell& cell);
  std::string remove_all_whitespace(std::string& str);
  std::string trim_whitespace(const std::string& str);
  std::string remove_first_and_last_whitespace(std::string& extracted_text);
  fz_rect scale_rect(fz_rect original, float factor);

  fz_pixmap* pixmap;
  fz_rect page_scoped_bounds;
  fz_rect page_bounds;
  unsigned char* samples;
  int width;
  int height;
  int stride;
  int components;
  std::vector<fz_rect> column_boundaries;
  std::vector<TableDataCell> cells;
};

void deleter_Datatable(void* p);
Result* classify_datatable(uint32_t page, fz_context* ctx, fz_document* doc);
Result* extract_datatable(uint32_t page, fz_context* ctx, fz_document* doc, void* shared);
