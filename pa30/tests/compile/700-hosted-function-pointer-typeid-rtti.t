#include <typeinfo>

typedef int (*function_pointer)();

const std::type_info & function_pointer_typeid_anchor()
{
  return typeid(function_pointer);
}

static_assert(sizeof(&function_pointer_typeid_anchor) > 0, "function pointer typeid anchor");
