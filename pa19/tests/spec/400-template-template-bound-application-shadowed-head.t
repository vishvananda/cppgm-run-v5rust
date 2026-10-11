// VALIDATION: compile-pass
// N3485 focus: 14.3.3 [temp.arg.template], 14.5.5.1 [temp.class.spec.match]
// A template-template parameter deduced from a partial-specialization match
// must keep the matched template identity when the same unqualified name is
// visible in the specialization body.

template<class T, class H1, class H2>
struct hash
{};

namespace library
{
  template<class T>
  struct hash
  {};

  namespace detail
  {
    struct text
    {};

    template<class T>
    text class_template_name()
    {
      return text();
    }

    template<class T>
    struct tn_holder
    {
      static int type_name()
      {
        return 1;
      }
    };

    template<template<class...> class L, class... T>
    struct tn_holder<L<T...> >
    {
      static int type_name()
      {
        text tn = class_template_name<L<T...> >();
        (void)tn;
        return 0;
      }
    };

    template<class T>
    int type_name()
    {
      return tn_holder<T>::type_name();
    }
  }
}

int main()
{
  typedef ::hash<int, char, long> hasher;
  return library::detail::type_name<hasher>();
}
