#pragma once

namespace lazy_qualified_shadow {
namespace api {

template<class T>
struct ctype
{
  enum mask
  {
    space = 1
  };

  bool is(mask, T value) const
  {
    return value != 0;
  }
};

}  // namespace api

template<class CharT>
int classify(CharT c)
{
  api::ctype<CharT> ctype;
  return ctype.is(api::ctype<CharT>::space, c) ? 0 : 1;
}

}  // namespace lazy_qualified_shadow
