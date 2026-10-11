#include <600-system-static-member-object-detector.h>
struct alloc { static void destroy(int *); };
static_assert(sizeof(std::destroy_traits<alloc>::select(*(alloc*)0, (int*)0)) == 1,
              "static member is callable through an object expression");
