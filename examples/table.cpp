#include "table.hpp"
#include <array>
#include <iostream>
#include <regex>
#include <string_view>

inline constexpr float EXPECTED_PAGE_HEIGHT = 840.0;
inline constexpr float FILL_PERCENTAGE_THRESHOLD = 0.70;
inline constexpr int VERTICAL_SEPERATOR_SCAN_WIDTH = 2;
inline constexpr int MAXIMUM_HORIONTAL_PIXEL_JUMP = 3;
inline constexpr int WHITE_PIXEL_THRESHOLD = 160;
inline constexpr float PIXMAP_SCALE = 2.0;
inline constexpr float UPPER_MARGIN_HEIGHT = 120.0;
inline constexpr float LOWER_MARGIN_HEIGHT = 80.0;
inline constexpr int MIN_EMPTY_LINES_FOR_CELL_SEPARATION = 3; // Require at least 3 empty lines to separate table members
inline constexpr int MIN_CONTENT_HEIGHT = 8;                  // Minimum height for a valid content block
inline constexpr float CELL_WALL_PADDING = 5.0;
inline constexpr float CELL_ROOF_PADDING = 5.0;
inline constexpr float VALID_PAGE_BOUND_MARGIN = 2.0;
inline constexpr std::array<std::string_view, 7> COLUMN_IDENTIFIERS = {
    "Keys", "DMC Army", "NATO stock number", "Item name", "Part No. / Dwg No.", "No. off", "Annotation (NSCM)"};

DataTable::DataTable(fz_context* ctx, fz_document* doc, uint32_t page) : Object(ctx, doc, page) {
  page_bounds = fz_bound_page(ctx, this->page);
  calc_page_scope();

  pixmap = fz_new_pixmap_from_page(ctx, this->page, fz_scale(PIXMAP_SCALE, PIXMAP_SCALE), fz_device_rgb(ctx), 0);
  if (!pixmap) {
    throw std::runtime_error("Failed to create pixmap from page");
  }

  samples = fz_pixmap_samples(ctx, pixmap);
  width = fz_pixmap_width(ctx, pixmap);
  height = fz_pixmap_height(ctx, pixmap);
  stride = fz_pixmap_stride(ctx, pixmap);
  components = fz_pixmap_components(ctx, pixmap);
}

DataTable::~DataTable() { fz_drop_pixmap(ctx, pixmap); }

Result* DataTable::valid_page_bounds() {
  // page_bounds is initialized in the constructor
  float diff = page_bounds.y1 - EXPECTED_PAGE_HEIGHT;
  if (diff <= VALID_PAGE_BOUND_MARGIN && diff >= -VALID_PAGE_BOUND_MARGIN) {
    return Result::ok(NULL, NULL);
  } else {
    return Result::fail(std::format("page height doesn't meet expected height {}", page_bounds.y1));
  }
}

std::string DataTable::remove_all_whitespace(std::string& str) {
  str.erase(std::remove(str.begin(), str.end(), ' '), str.end());

  return str;
}

std::string DataTable::trim_whitespace(const std::string& str) {
  if (str.empty())
    return "";

  size_t start = 0;
  while (start < str.length() && std::isspace(static_cast<unsigned char>(str[start]))) {
    start++;
  }

  if (start == str.length())
    return "";

  size_t end = str.length() - 1;
  while (end > start && std::isspace(static_cast<unsigned char>(str[end]))) {
    end--;
  }

  return str.substr(start, end - start + 1);
}

TableDataKeyCell DataTable::parse_key_column(const std::string& text, TableDataCell cell) {
  TableDataKeyCell key_cell;
  static_cast<TableDataCell&>(key_cell) = std::move(cell);

  std::string cleaned_text = trim_whitespace(text);

  if (cleaned_text.empty()) {
    key_cell.key = 0;
    key_cell.included = false;
    return key_cell;
  }

  // Parse patterns like "1", "NI2", "3", etc.
  std::regex key_pattern(R"(^(N[Il])?(\d+)$)");
  std::smatch matches;

  if (std::regex_match(cleaned_text, matches, key_pattern)) {
    try {
      // Extract the number part (group 2)
      std::string number_str = matches[2].str();
      key_cell.key = std::stoi(number_str);

      // Check if it's a "NI" (Not Included) item (group 1)
      std::string prefix = matches[1].str();
      key_cell.included = prefix.empty(); // If no prefix, it's included

    } catch (const std::exception& e) {
      key_cell.key = 0;
      key_cell.included = false;
    }
  } else {
    std::regex number_only(R"(\d+)");
    std::smatch number_match;
    if (std::regex_search(cleaned_text, number_match, number_only)) {
      try {
        key_cell.key = std::stoi(number_match[0].str());
        key_cell.included = true; // Assume included if no "NI" prefix
      } catch (const std::exception& e) {
        key_cell.key = 0;
        key_cell.included = false;
      }
    } else {
      key_cell.key = 0;
      key_cell.included = false;
    }
  }

  return key_cell;
}

TableDataCountCell DataTable::parse_count_column(const std::string& text, TableDataCell cell) {
  TableDataCountCell count_cell;
  static_cast<TableDataCell&>(count_cell) = std::move(cell);

  try {
    if (!text.empty()) {
      count_cell.count = std::stoi(text);
    }
  } catch (...) {
    count_cell.count = 1;
  }

  return count_cell;
}

std::vector<TableCellKind> DataTable::extract_cells() {
  calc_column_boundaries();
  if (column_boundaries.empty()) {
    throw std::runtime_error("no column boundaries detected — page likely lacks expected table structure");
  }

  fz_rect fig_key_boundary = column_boundaries[0];
  int start_x = static_cast<int>((fig_key_boundary.x0 + 5.0f) * 2.0f);
  int end_x = static_cast<int>((fig_key_boundary.x1 - 2.5f) * 2.0f);
  int start_y = static_cast<int>(fig_key_boundary.y0 * 2.0f);
  int end_y = static_cast<int>(fig_key_boundary.y1 * 2.0f);

  CellVerticalBoundaryList cell_vertical_boundaries = calc_cell_vertical_boundaires(start_y, end_y, start_x, end_x);
  construct_cells(cell_vertical_boundaries);

  std::vector<TableCellKind> result;
  result.reserve(cells.size());

  for (auto& item : cells) {
    TableDataCell cell = std::move(item);
    std::string text = extract_text_from_cell(cell);
    CellColumn column = static_cast<CellColumn>(cell.column);

    switch (column) {
    case KEY:
      result.emplace_back(std::move(parse_key_column(remove_all_whitespace(text), std::move(cell))));
      continue;
    case NUM_OFF:
      result.emplace_back(std::move(parse_count_column(remove_all_whitespace(text), std::move(cell))));
      continue;
    case DMC_ARMY:
    case PART_NUM:
    case ANNOTATION:
    case NATO_STOCK_NUM:
      text = remove_all_whitespace(text);
      break;
    case ITEM_NAME:
      text = remove_first_and_last_whitespace(text);
      break;
    }

    cell.text = std::make_unique<std::string>(text);
    result.emplace_back(std::move(cell));
  }

  return result;
}

std::string DataTable::remove_first_and_last_whitespace(std::string& extracted_text) {
  if (!extracted_text.empty() && extracted_text[0] == ' ') {
    extracted_text.erase(0, 1); // Remove first character
  }
  if (!extracted_text.empty() && extracted_text[extracted_text.length() - 1] == ' ') // Check last valid index
  {
    extracted_text.erase(extracted_text.length() - 1, 1); // Remove last character
  }

  return extracted_text;
}

std::string DataTable::extract_text_from_cell(const TableDataCell& cell) {
  std::string result;
  fz_stext_page* stext = nullptr;
  fz_stext_options opts = {0};

  fz_try(ctx) {
    stext = fz_new_stext_page_from_page(ctx, page, &opts);

    for (fz_stext_block* block = stext->first_block; block; block = block->next) {
      if (block->type != FZ_STEXT_BLOCK_TEXT)
        continue;

      for (fz_stext_line* line = block->u.t.first_line; line; line = line->next) {
        std::string lineText;
        bool hasCharsInBox = false;

        // Check each character in the line
        for (fz_stext_char* ch = line->first_char; ch; ch = ch->next) {
          fz_rect char_bbox = fz_rect_from_quad(ch->quad);

          float cx = (char_bbox.x0 + char_bbox.x1) * 0.5f;
          float cy = (char_bbox.y0 + char_bbox.y1) * 0.5f;
          if (cx >= cell.boundary.x0 && cx <= cell.boundary.x1 && cy >= cell.boundary.y0 && cy <= cell.boundary.y1) {
            if (ch->c >= 32) { // Printable character
              lineText += static_cast<char>(ch->c);
            }
            hasCharsInBox = true;
          }
        }

        // Add line to result if it had characters in the box
        if (hasCharsInBox && !lineText.empty()) {
          if (!result.empty())
            result += '\n';
          result += lineText;
        }
      }
    }
  }
  fz_always(ctx) {
    if (stext)
      fz_drop_stext_page(ctx, stext);
  }
  fz_catch(ctx) {
    const char* error_msg = fz_caught_message(ctx);
    throw std::runtime_error("failed to extract text from cell " + std::string(error_msg));
  }

  if (!result.empty() && result[0] == ' ') {
    result.erase(0, 1);
  }
  if (!result.empty() && result[result.length() - 1] == ' ') {
    result.erase(result.length() - 1, 1);
  }

  result.erase(std::remove(result.begin(), result.end(), '\''), result.end());
  result.erase(std::remove(result.begin(), result.end(), '.'), result.end());
  result.erase(std::remove(result.begin(), result.end(), '\n'), result.end());

  return result;
}

void DataTable::calc_page_scope() {
  float top_left_x = page_bounds.x0;
  float top_left_y = page_bounds.y0 + UPPER_MARGIN_HEIGHT;
  float bottom_right_x = page_bounds.x1;
  float bottom_right_y = page_bounds.y1 - LOWER_MARGIN_HEIGHT;

  page_scoped_bounds = {top_left_x, top_left_y, bottom_right_x, bottom_right_y};
}

fz_rect DataTable::scale_rect(fz_rect original, float factor) {
  return {original.x0 * factor, original.y0 * factor, original.x1 * factor, original.y1 * factor};
}

void DataTable::calc_column_boundaries() {
  std::vector<int> vertical_seperators = calc_column_seperators(scale_rect(page_scoped_bounds, 2.0));

  for (size_t i = 0; i < COLUMN_IDENTIFIERS.size() && i + 1 < vertical_seperators.size(); ++i) {
    fz_rect data_bb;

    data_bb.x0 = vertical_seperators[i] / PIXMAP_SCALE;
    data_bb.x1 = vertical_seperators[i + 1] / PIXMAP_SCALE;

    data_bb.y0 = page_scoped_bounds.y0;
    data_bb.y1 = page_scoped_bounds.y1;

    column_boundaries.emplace_back(data_bb);
  }
}

CellVerticalBoundaryList DataTable::calc_cell_vertical_boundaires(int start_y, int end_y, int start_x, int end_x) {
  bool in_content = false;
  int content_start_y = -1;
  int consecutive_empty_lines = 0;
  CellVerticalBoundaryList cell_vertical_boundaries = {};
  std::vector<std::pair<int, int>> content_blocks;

  // Scan downwards line by line to find content blocks
  for (int y = start_y; y < end_y; ++y) {
    bool line_has_content = row_has_content(y, start_x, end_x);

    if (!in_content && line_has_content) {
      // Found start of new content block
      in_content = true;
      content_start_y = y;
      consecutive_empty_lines = 0;
    } else if (in_content && !line_has_content) {
      // Found empty line within content - count consecutive empty lines
      consecutive_empty_lines++;

      // Only end content block if we have enough consecutive empty lines
      if (consecutive_empty_lines >= MIN_EMPTY_LINES_FOR_CELL_SEPARATION) {
        // Found end of content block
        in_content = false;
        int content_end_y = y - consecutive_empty_lines; // Last line with content

        // Check if content block is tall enough
        int content_height = content_end_y - content_start_y + 1;
        if (content_height >= MIN_CONTENT_HEIGHT) {
          content_blocks.emplace_back(content_start_y, content_end_y);
        }

        consecutive_empty_lines = 0;
      }
    } else if (in_content && line_has_content) {
      // Found content line again - reset empty line counter
      consecutive_empty_lines = 0;
    }
  }

  // Handle final content block if we're still in one
  if (in_content) {
    int content_end_y = end_y - 1;
    int content_height = content_end_y - content_start_y + 1;
    if (content_height >= MIN_CONTENT_HEIGHT) {
      content_blocks.emplace_back(content_start_y, content_end_y);
    }
  }

  // Now convert content blocks to boundaries with adjacent positioning
  for (size_t i = 0; i < content_blocks.size(); ++i) {
    float top_y = content_blocks[i].first / 2.0f; // Convert to page coordinates
    float bottom_y;

    if (i + 1 < content_blocks.size()) {
      // Not the last block - bottom is 5 pixels above the next block's top
      float next_top_y = content_blocks[i + 1].first;
      bottom_y = (next_top_y - CELL_ROOF_PADDING) / 2.0f;
    } else {
      // Last block - extend to near the end of the scan area
      bottom_y = (end_y - CELL_ROOF_PADDING) / 2.0f; // Leave 5 pixels before the border
    }

    cell_vertical_boundaries.emplace_back(top_y, bottom_y);
  }

  return cell_vertical_boundaries;
}

void DataTable::construct_cells(const CellVerticalBoundaryList& cell_vertical_boundaries) {
  for (size_t col = 0; col < column_boundaries.size(); ++col) {
    fz_rect column = column_boundaries[col];

    for (size_t row = 0; row < cell_vertical_boundaries.size(); ++row) {
      std::pair<float, float> vert_boundary = cell_vertical_boundaries[row];

      TableDataCell cell;
      cell.row_num = static_cast<int>(row + 1);
      cell.column = static_cast<int>(col + 1);
      cell.boundary.x0 = column.x0;
      cell.boundary.y0 = vert_boundary.first;
      cell.boundary.x1 = column.x1;
      cell.boundary.y1 = vert_boundary.second;
      cell.text = nullptr;

      int width = cell.boundary.x1 - cell.boundary.x0;
      int height = cell.boundary.y1 - cell.boundary.y0;
      if (width > 0 && height) {
        cells.emplace_back(std::move(cell));
      }
    }
  }
}

bool DataTable::row_has_content(int y, int start_x, int end_x) {
  for (int x = start_x; x <= end_x; ++x) {
    if (x >= 0 && x < width && y >= 0 && y < height) {
      unsigned char* pixel = samples + (y * stride) + (x * components);
      if (!is_white_pixel(pixel)) {
        return true;
      }
    }
  }
  return false;
}

std::vector<int> DataTable::calc_column_seperators(fz_rect scan_bounds) {
  std::vector<int> column_separators = {}; // !

  bool in_separator = false;
  int separator_start = -1;
  int pixels_scanned = 0;
  int filled_pixels = 0;

  for (int x = scan_bounds.x0; x <= scan_bounds.x1; x += VERTICAL_SEPERATOR_SCAN_WIDTH) {
    bool is_filled = is_vertical_line_filled(x, scan_bounds.y0, scan_bounds.y1);
    pixels_scanned++;
    if (is_filled)
      filled_pixels++;

    if (is_filled && !in_separator) {
      // Found start of a separator group
      in_separator = true;
      separator_start = x;
    } else if (!is_filled && in_separator) {
      // End of separator group - record the middle of the group
      int separator_width = x - separator_start;
      if (separator_width >= VERTICAL_SEPERATOR_SCAN_WIDTH) {
        int separator_center = separator_start + separator_width / 2;
        column_separators.push_back(separator_center);
      }
      in_separator = false;
    }
  }

  // Handle case where last group extends to the end
  if (in_separator) {
    int separator_width = scan_bounds.x1 - separator_start;
    if (separator_width >= VERTICAL_SEPERATOR_SCAN_WIDTH) {
      int separator_center = separator_start + separator_width / 2;
      column_separators.push_back(separator_center);
    }
  }

  return column_separators;
}

bool DataTable::is_vertical_line_filled(int x, int start_y, int end_y) {
  int total_lines = 0;
  int lines_with_content = 0;
  int current_x = x;

  for (int y = start_y; y <= end_y; ++y) {
    total_lines++;

    int best_x = current_x;
    int min_distance = MAXIMUM_HORIONTAL_PIXEL_JUMP + 1;
    bool found_content_on_line = false;

    for (int dx = -MAXIMUM_HORIONTAL_PIXEL_JUMP; dx <= MAXIMUM_HORIONTAL_PIXEL_JUMP; ++dx) {
      if (y < 0 || y >= height) {
        continue;
      }

      int check_x = current_x + dx;
      if (check_x < 0 || check_x >= width) {
        continue;
      }

      unsigned char* pixel = samples + (y * stride) + (check_x * components);
      if (is_white_pixel(pixel)) {
        continue;
      }

      int distance = abs(dx);
      if (distance < min_distance) {
        min_distance = distance;
        best_x = check_x;
        found_content_on_line = true;
      }
    }

    if (found_content_on_line) {
      lines_with_content++;
      current_x = best_x;
    }
  }

  if (total_lines == 0)
    return false;

  double fill_percentage = (double)lines_with_content / total_lines;
  return fill_percentage >= FILL_PERCENTAGE_THRESHOLD;
}

bool DataTable::is_white_pixel(unsigned char* pixel) {
  for (int i = 0; i < components; ++i) {
    if (pixel[i] < WHITE_PIXEL_THRESHOLD) {
      return false;
    }
  }
  return true;
}

void deleter_Datatable(void* p) { delete static_cast<DataTable*>(p); };

Result* classify_datatable(uint32_t page, fz_context* ctx, fz_document* doc) {
  auto inst = std::make_unique<DataTable>(ctx, doc, page);

  UNWRAP_RESULT(inst->valid_page_bounds());

  return Result::ok(inst.release(), deleter_Datatable);
};

Result* extract_datatable(uint32_t page, fz_context* ctx, fz_document* doc, void* shared) {
  DataTable* inst = static_cast<DataTable*>(shared);

  try {
    std::vector<TableCellKind> cells = inst->extract_cells();
    nlohmann::json j = nlohmann::json{{"table_data", cells}};

    return json_to_payload(j);
  } catch (const std::exception& e) {
    return Result::fail(std::format("failed to extract datatable {}", e.what()));
  }
};
