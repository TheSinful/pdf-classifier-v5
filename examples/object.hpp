#pragma once

#include <format>
#include <mupdf/fitz.h>
#include <mupdf/pdf.h>
#include <string>
#include <vector>

struct PdfText {
  std::string text;
  std::string font_name;
  float font_size;
  fz_rect bbox;
};

template <bool EnableTextExtraction = true, bool EnableTextCompression = EnableTextExtraction> class Object {
public:
  Object(fz_context* ctx, fz_document* doc, uint32_t page_num)
      : ctx(ctx), doc(doc), page_num(page_num), checked_img(false), page(fz_load_page(ctx, doc, page_num)) {
    if constexpr (EnableTextExtraction) {
      extract_text();
      if constexpr (EnableTextCompression) {
        compress_text();
      }
    }
  }

  ~Object() { fz_drop_page(ctx, page); }

protected:
  std::string compressed_text;
  std::vector<PdfText> extracted_text;
  fz_context* ctx;
  fz_document* doc;
  fz_page* page;
  uint32_t page_num;

  int frequency_of(const std::string& substr, const std::string& within, int max_errors) {
    if (substr.empty())
      return 0;

    int count = 0;
    const int substr_len = static_cast<int>(substr.length());
    const int str_len = static_cast<int>(within.length());

    for (size_t i = 0; i <= str_len - substr_len; ++i) {
      std::string window = within.substr(i, substr_len);
      if (levenshtein_distance(substr, window) <= max_errors) {
        count++;
      }
    }

    return count;
  }

  bool contains_text(const std::string& substr, const std::string& str) {
    return str.find(substr) != std::string::npos;
  }

  bool has_image() {
    if (checked_img) {
      return true;
    }

    fz_try(ctx) {
      pdf_page* pdf_page = pdf_page_from_fz_page(ctx, page);
      if (!pdf_page) {
        break;
      }

      pdf_obj* resources = pdf_page_resources(ctx, pdf_page);
      if (!resources) {
        break;
      }

      pdf_obj* xobject = pdf_dict_get(ctx, resources, PDF_NAME(XObject));
      if (!xobject) {
        break;
      }

      int n = pdf_dict_len(ctx, xobject);
      for (int i = 0; i < n; i++) {
        pdf_obj* obj = pdf_dict_get_val(ctx, xobject, i);
        pdf_obj* subtype = pdf_dict_get(ctx, obj, PDF_NAME(Subtype));

        if (pdf_name_eq(ctx, subtype, PDF_NAME(Image))) {
          return true;
        }
      }
    }
    fz_catch(ctx) {
      throw std::runtime_error(std::format("Error checking page resources for images: {}", fz_caught_message(ctx)));
    }

    return false;
  }

private:
  bool checked_img;

  void extract_text() {
    extracted_text = {};

    fz_try(ctx) {
      fz_stext_page* stext_page;
      fz_stext_options opts = {0};

      stext_page = fz_new_stext_page_from_page(ctx, page, &opts); // only returns NULL when this->page is NULL
      if (!stext_page) {
        fz_throw(ctx, FZ_ERROR_GENERIC, "no page object exists");
      }

      int block_count = 0;
      int text_block_count = 0;

      for (fz_stext_block* block = stext_page->first_block; block; block = block->next) {
        block_count++;

        if (block->type != FZ_STEXT_BLOCK_TEXT) {
          continue;
        }

        text_block_count++;

        int line_count = 0;
        for (fz_stext_line* line = block->u.t.first_line; line; line = line->next) {
          line_count++;

          std::vector<fz_stext_char*> line_chars;
          for (fz_stext_char* ch = line->first_char; ch; ch = ch->next) {
            line_chars.emplace_back(ch);
          }

          if (line_chars.empty()) {
            continue;
          }

          size_t start = 0;
          int text_run_count = 0;

          for (size_t i = 1; i <= line_chars.size(); ++i) {
            bool end_of_run = (i == line_chars.size()) || (line_chars[i]->font != line_chars[start]->font) ||
                              (line_chars[i]->size != line_chars[start]->size);

            if (!end_of_run)
              continue;

            text_run_count++;
            PdfText entry;

            std::string text_run;
            for (size_t j = start; j < i; ++j) {
              text_run += static_cast<char>(line_chars[j]->c);
            }

            entry.text = text_run;
            entry.font_name = fz_font_name(ctx, line_chars[start]->font);
            entry.font_size = line_chars[start]->size;
            entry.bbox = fz_rect_from_quad(line_chars[start]->quad);

            extracted_text.emplace_back(entry);
            start = i;
          }
        }
      }
    }
    fz_catch(ctx) {
      throw std::runtime_error(
          std::format("failed to extract text from page {}, {}", page_num, fz_caught_message(ctx)));
    }
  }

  void compress_text() {
    if (extracted_text.empty()) {
      return;
    }

    size_t length = 0;
    for (const auto& entry : extracted_text) {
      length += entry.text.length();

      if (length > 0)
        length += 1; // Add separator
    }

    if (length > 0)
      length -= 1; // Remove separator

    for (const PdfText& entry : extracted_text) {
      compressed_text.append(entry.text + '\n');
    }
  }

  int levenshtein_distance(const std::string& s1, const std::string& s2) {
    const int len1 = static_cast<int>(s1.length());
    const int len2 = static_cast<int>(s2.length());

    std::vector<std::vector<int>> dp(len1 + 1, std::vector<int>(len2 + 1));

    for (int i = 0; i <= len1; ++i)
      dp[i][0] = i;
    for (int j = 0; j <= len2; ++j)
      dp[0][j] = j;

    for (int i = 1; i <= len1; ++i) {
      for (int j = 1; j <= len2; ++j) {
        if (s1[i - 1] == s2[j - 1]) {
          dp[i][j] = dp[i - 1][j - 1];
        } else {
          dp[i][j] = 1 + std::min({dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1]});
        }
      }
    }

    return dp[len1][len2];
  }
};