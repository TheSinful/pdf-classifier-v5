#include "test.hpp"
#include <iostream>
#include <shared/result.h>
#include <mupdf/fitz.h>
#include <thread>

struct MySharedData
{
    int page_count;
    double confidence;
};

/*
    Deleters are utilized because the Rust engine does not know how SharedData was allocated,
    To ensure safety on the Rust side we have to ensure Drop is safe, therefore the user
    has to specify directions to free shared. or rather, the user themselves free it.
    Although, ofcourse you don't have to utilize the weird function naming that I do in this example,
    I personally think keeping it as explicit as possible is the best approach since it is the only
    real way for the classifier itself to have an unexpected/silent problem.
*/
static void deleter_MySharedData(void *p)
{
    delete static_cast<MySharedData *>(p);
}

Result *classify(uint32_t page, fz_context *ctx, fz_document *doc)
{
    std::this_thread::sleep_for(std::chrono::seconds(3));

    MySharedData *data = new MySharedData{42, 0.95};
    return Result::ok(data, deleter_MySharedData);
}

Result *extract(uint32_t page, fz_context *ctx, fz_document *doc, void *shared)
{
    MySharedData *data = reinterpret_cast<MySharedData *>(shared);

    if (data->page_count != 42 || data->confidence != 0.95)
    {
        return Result::fail("internal shared data differs from what was provided.");
    }
    else
    {
        return Result::ok(NULL, NULL);
    }
}
