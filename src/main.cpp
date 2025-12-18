#include <shared/generated_page_types.h>
#include "headers/object.h"

int main()
{
    ObjectFactory factory; 
    std::shared_ptr<Object> root = factory.create(); 

    return 0;
}