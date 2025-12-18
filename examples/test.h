#pragma once

#include <any>
#include <context.h>

void* classify(const PageContext& ctx);
void extract(const PageContext& ctx, void* shared);