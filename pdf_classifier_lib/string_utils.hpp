#pragma once

#include <string>
#include <vector>
#include <nlohmann/json.hpp>
#include <mupdf/fitz.h>

inline std::string as_string(std::string s) { return s; }
inline std::string as_string(std::string_view s) { return std::string(s); }
inline std::string as_string(const char *s) { return std::string(s); }
inline std::string as_string(const nlohmann::json &j) { return j.dump(); }

template <class T>
concept IntoString = requires(T &&t) {
    { as_string(std::forward<T>(t)) } -> std::same_as<std::string>;
};

struct PdfText
{
    std::string text;
    std::string font_name;
    float font_size;
    fz_rect bbox;
};

std::vector<PdfText> extract_text(fz_context *ctx, fz_page *page, uint32_t page_num);
int frequency_of(const std::string &substr, const std::string &within, int max_errors);
bool has_image(fz_context *ctx, fz_page *page);
std::string compress_text(std::vector<PdfText> extracted_text);
int levenshtein_distance(const std::string &s1, const std::string &s2);
inline bool contains_text(const std::string &substr, const std::string &str) { return str.find(substr) != std::string::npos; }
inline void deleter_StdString(void *ptr) { delete static_cast<std::string *>(ptr); }