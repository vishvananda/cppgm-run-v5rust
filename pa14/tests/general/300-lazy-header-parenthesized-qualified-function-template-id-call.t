#include "../lazy_parenthesized_qualified_function_template_id_call.h"

int main()
{
  return LazyTemplateIdUser::TemplateIdBox<int>::f(3, 2) == 2 ? 0 : 1;
}
