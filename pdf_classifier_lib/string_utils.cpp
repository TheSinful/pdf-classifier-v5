#include "string_utils.hpp"
#include "wrappers.hpp"

#include <algorithm>
#include <format>
#include <mupdf/pdf.h>

// Every MuPDF call goes through fz_call, so each fz_try region contains exactly
// one C call and no C++ objects. That matters because longjmp does not run
// destructors - anything constructed inside a guarded region leaks when MuPDF
// errors out. Keeping the regions to a single call makes that impossible.

std::vector<PdfText> extract_text(fz_context *ctx, fz_page *page, uint32_t page_num)
{
    std::vector<PdfText> extracted_text;

    if (!page)
        throw FzError(FZ_ERROR_GENERIC,
                      std::format("no page to extract text from (page {})", page_num).c_str());

    fz_stext_options opts = {0};

    // Owning handle - the original leaked this on every call, including the
    // error path.
    FzSTextPage stext = FzSTextPage::make(ctx, fz_new_stext_page_from_page, page, &opts);

    if (!stext)
        throw FzError(FZ_ERROR_GENERIC,
                      std::format("no structured text for page {}", page_num).c_str());

    // Walking the structure is plain pointer traversal - no MuPDF calls, so no
    // guarding needed from here down.
    for (fz_stext_block *block = stext.get()->first_block; block; block = block->next)
    {
        if (block->type != FZ_STEXT_BLOCK_TEXT)
            continue;

        for (fz_stext_line *line = block->u.t.first_line; line; line = line->next)
        {
            std::vector<fz_stext_char *> line_chars;
            for (fz_stext_char *ch = line->first_char; ch; ch = ch->next)
                line_chars.emplace_back(ch);

            if (line_chars.empty())
                continue;

            size_t start = 0;

            for (size_t i = 1; i <= line_chars.size(); ++i)
            {
                const bool end_of_run = (i == line_chars.size()) ||
                                        (line_chars[i]->font != line_chars[start]->font) ||
                                        (line_chars[i]->size != line_chars[start]->size);

                if (!end_of_run)
                    continue;

                std::string text_run;
                text_run.reserve(i - start);

                for (size_t j = start; j < i; ++j)
                    text_run += static_cast<char>(line_chars[j]->c);

                PdfText entry;
                entry.text = std::move(text_run);
                entry.font_name = fz_font_name(ctx, line_chars[start]->font);
                entry.font_size = line_chars[start]->size;
                entry.bbox = fz_rect_from_quad(line_chars[start]->quad);

                extracted_text.emplace_back(std::move(entry));
                start = i;
            }
        }
    }

    return extracted_text;
}

bool has_image(fz_context *ctx, fz_page *page)
{
    if (!page)
        return false;

    pdf_page *ppage = fz_call(ctx, pdf_page_from_fz_page, page);
    if (!ppage)
        return false;

    pdf_obj *resources = fz_call(ctx, pdf_page_resources, ppage);
    if (!resources)
        return false;

    pdf_obj *xobject = fz_call(ctx, pdf_dict_get, resources, PDF_NAME(XObject));
    if (!xobject)
        return false;

    const int n = fz_call(ctx, pdf_dict_len, xobject);

    for (int i = 0; i < n; ++i)
    {
        pdf_obj *obj = fz_call(ctx, pdf_dict_get_val, xobject, i);
        pdf_obj *subtype = fz_call(ctx, pdf_dict_get, obj, PDF_NAME(Subtype));

        if (fz_call(ctx, pdf_name_eq, subtype, PDF_NAME(Image)))
            return true;
    }

    return false;
}

std::string compress_text(std::vector<PdfText> extracted_text)
{
    if (extracted_text.empty())
        return "";

    size_t length = 0;
    for (const PdfText &entry : extracted_text)
        length += entry.text.length() + 1;

    std::string compressed_text;
    compressed_text.reserve(length);

    for (const PdfText &entry : extracted_text)
    {
        compressed_text.append(entry.text);
        compressed_text.push_back('\n');
    }

    return compressed_text;
}

int frequency_of(const std::string &substr, const std::string &within, int max_errors)
{
    if (substr.empty() || within.size() < substr.size())
        return 0;

    int count = 0;
    const size_t substr_len = substr.length();

    for (size_t i = 0; i + substr_len <= within.size(); ++i)
    {
        std::string window = within.substr(i, substr_len);
        if (levenshtein_distance(substr, window) <= max_errors)
            ++count;
    }

    return count;
}

int levenshtein_distance(const std::string &s1, const std::string &s2)
{
    const int len1 = static_cast<int>(s1.length());
    const int len2 = static_cast<int>(s2.length());

    std::vector<std::vector<int>> dp(len1 + 1, std::vector<int>(len2 + 1));

    for (int i = 0; i <= len1; ++i)
        dp[i][0] = i;
    for (int j = 0; j <= len2; ++j)
        dp[0][j] = j;

    for (int i = 1; i <= len1; ++i)
    {
        for (int j = 1; j <= len2; ++j)
        {
            if (s1[i - 1] == s2[j - 1])
                dp[i][j] = dp[i - 1][j - 1];
            else
                dp[i][j] = 1 + std::min({dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1]});
        }
    }

    return dp[len1][len2];
}
