#pragma once

namespace LazyTemplateIdLib {

template<class T>
T template_id_min(T a, T b)
{
  return a < b ? a : b;
}

}
