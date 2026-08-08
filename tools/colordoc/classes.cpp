#include "classes.hpp"
#include "util.hpp"
#include <memory>

/// Every class classifies identically: render the page, demand its color.
/// On success the sampled `ColorPage` becomes the shared payload so the
/// matching extract does not have to render a second time.
ClassificationResult ColorObject::classify(Attached &att)
{
    auto inst = std::make_unique<ColorPage>();

    Result *res = inst->expect(att, expected_, name_);

    if (res->type == Result::Type::FAIL)
    {
        return ClassificationResult::fail(res->fail_rsn);
    }
    else
    {
        return ClassificationResult::ok();
    }
}

ExtractionResult ColorObject::extract(Attached &att)
{
    return ExtractionResult::ok(nlohmann::json{
        {"page", att.page_num()},
        {"class", name_},
        {"rgb", {static_cast<int>(color_.r), static_cast<int>(color_.g), static_cast<int>(color_.b)}},
    });
}
