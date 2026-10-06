#include "diagram.hpp"

ClassificationResult Diagram::contains_valid_chapter_text() {
  if (frequency_of("Chapter", compressed_text, 1) == EXPECTED_LOWER_CHAPTER &&
      frequency_of("CHAPTER", compressed_text, 1) == EXPECTED_UPPER_CHAPTER) {
    return ClassificationResult::ok();
  } else {
    return ClassificationResult::fail("doesn't contain valid chapter text");
  }
}

ClassificationResult Diagram::contains_valid_figure_text() {
  std::smatch matches;

  if (std::regex_search(compressed_text, matches, FIG_NUM_PATTERN)) {
    fig_num = std::stoi(matches[1].str());
    caption = matches[2].str();
    return ClassificationResult::ok();
  } else {
    return ClassificationResult::fail("failed to find figure text");
  }
}

ClassificationResult Diagram::contains_image(Attached& att) {
  if (att.has_image()) {
    return ClassificationResult::ok();
  } else {
    return ClassificationResult::fail("doesn't contain an image.");
  }
}

ClassificationResult Diagram::classify(Attached& att) {
  UNWRAP(contains_image(att));
  UNWRAP(contains_valid_chapter_text());
  UNWRAP(contains_valid_figure_text());

  return ClassificationResult::ok();
}

ExtractionResult Diagram::extract(Attached&) {
  return ExtractionResult::ok(nlohmann::json{{"fig_num", fig_num}, {"caption", caption}});
}


