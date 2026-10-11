// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], dependent decltype in conversion-function SFINAE.

#include "../support.h"

template<class T, class U>
struct has_get
{
  typedef decltype(declval<T>().template get<U>()) detected;
  static const bool value = true;
};

enum value_kind
{
  value_null
};

struct json_like
{
  template<class V,
           typename enable_if<has_get<const json_like &, V>::value, int>::type = 0>
  operator V() const
  {
    return get<V>();
  }

  template<class V>
  V get() const
  {
    return V();
  }
};

value_kind convert(json_like value)
{
  return value;
}

int main()
{
  json_like value;
  return convert(value) == value_null ? 0 : 1;
}
