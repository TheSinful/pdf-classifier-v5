#include "test.h"
#include <iostream>

void *classify(fz_context *ctx, fz_document *doc)
{
    return (void *)0x1;
}

void extract(fz_context *ctx, fz_document *doc, void *shared)
{
    return;
}