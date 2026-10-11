#pragma once

#include "lazy_parenthesized_qualified_function_template_id_lib.h"

namespace LazyTemplateIdUser {

template<class T>
struct TemplateIdBox {
  static T f(T a, T b)
  {
    return (LazyTemplateIdLib::template_id_min<T>)(a, b);
  }
};

}
