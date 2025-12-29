#include "test.h"
#include <iostream>

struct MySharedData
{
    int page_count;
    double confidence;
};

static void deleter_MySharedData(void *p)
{
    delete static_cast<MySharedData *>(p);
}

Result *classify(fz_context *ctx, fz_document *doc)
{
    MySharedData *data = new MySharedData{42, 0.95};
    Result *res = Result::ok(data, deleter_MySharedData);
    return res;
}

Result *extract(fz_context *ctx, fz_document *doc, void *shared)
{
    Result *res = Result::ok(NULL, NULL); // currently doesn't return anything special
    return res;
}