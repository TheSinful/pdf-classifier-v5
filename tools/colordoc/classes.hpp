#pragma once

#include "color_page.hpp"
#include <cstdint>
#include <mupdf/fitz.h>
#include <shared/result.h>
#include <pdf_classifier_lib/object.hpp>

/// The colordoc schema, mirroring the shape of the reference project in
/// `examples/`:
///
///     section                 root, organizational
///     +- subsection           organizational
///        +- figure            dependent, first in pair
///        +- caption           dependent, second in pair
///
/// Every classify function is the same test - "is this page my color?" - so any
/// disagreement with the ground truth emitted by `generate_doc.py` is an engine
/// bug rather than a classification ambiguity.

class ColorObject : public Object
{
public:
    ColorObject(uint32_t page, Rgb expected, const char *name)
        : Object(page), expected_(expected), name_(name) {}

    ClassificationResult classify(Attached &att) override;
    ExtractionResult extract(Attached &att) override;

protected:
    Rgb expected_;
    const char *name_;
    Rgb color_{};
};

class Section : public ColorObject
{
    explicit Section(uint32_t page) : ColorObject(page, SECTION_COLOR, "section") {}
};
class SubSection : public ColorObject
{
    explicit SubSection(uint32_t page) : ColorObject(page, SUBSECTION_COLOR, "subsection") {}
};
class Figure : public ColorObject
{
    explicit Figure(uint32_t page) : ColorObject(page, FIGURE_COLOR, "figure") {}
};
class Caption : public ColorObject
{
    explicit Caption(uint32_t page) : ColorObject(page, CAPTION_COLOR, "caption") {}
};