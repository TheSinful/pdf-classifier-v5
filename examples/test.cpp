#include "test.h"
#include <iostream>

struct MySharedData
{
    int page_count;
    double confidence;
};

/*
    Deleters are utilized because the Rust engine does not know how SharedData was allocated, 
    To ensure safety on the Rust side we have to ensure Drop is safe, therefore the extra 
    boilerplate is worth it. 
*/
static void deleter_MySharedData(void *p)
{
    delete static_cast<MySharedData *>(p);
}

Result *classify(uint32_t page, fz_context *ctx, fz_document *doc)
{
    MySharedData *data = new MySharedData{42, 0.95};
    Result *res = Result::ok(data, deleter_MySharedData);
    return res;
}

Result *extract(uint32_t page, fz_context *ctx, fz_document *doc, void *shared)
{
    Result *res = Result::ok(NULL, NULL);

    return res;
}
