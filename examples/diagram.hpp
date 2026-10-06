#pragma once

#include <mupdf/fitz.h>
#include <pdf_classifier_lib/object.hpp>
#include <pdf_classifier_lib/result.hpp>
#include <regex>
#include <shared/result.h>

inline const std::regex FIG_NUM_PATTERN(R"(Fig\s+(\d+):\s*(.+))");
inline constexpr int EXPECTED_UPPER_CHAPTER = 0;
inline constexpr int EXPECTED_LOWER_CHAPTER = 1;

class Diagram : public ObjectWith<TextExtraction> {
public:
  explicit Diagram(int page, Attached& att) : ObjectWith(page, att) {}

  ClassificationResult classify(Attached& att) override;
  ExtractionResult extract(Attached& att) override;

  ClassificationResult evaluate_capability_failures(const std::vector<CapabilityFailure> &failures) override {
    return ClassificationResult::fail(failures.front().reason); 
  }

  int fig_num = 0;
  std::string caption{"uninitialized"};

private:
  ClassificationResult contains_valid_chapter_text();
  ClassificationResult contains_valid_figure_text();
  ClassificationResult contains_image(Attached& att);
};

DEFINE_OBJECT(diagram, Diagram);
