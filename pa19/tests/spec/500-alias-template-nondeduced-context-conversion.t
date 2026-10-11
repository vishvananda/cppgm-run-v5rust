// VALIDATION: compile-pass
// N3485 focus: 14.8.2.5 [temp.deduct.type]
// An alias whose target is a dependent qualified member type remains a
// non-deduced context, so earlier arguments can determine the conversion type.

template<class T>
struct type_identity
{
  typedef T type;
};

template<class T>
using type_identity_t = typename type_identity<T>::type;

template<class T>
struct view
{
  view(const T *)
  {
  }
};

template<class T>
int equal(view<T>, type_identity_t<view<T> >)
{
  return 1;
}

template<class T>
int identity_only(type_identity_t<T>)
{
  return 1;
}

int identity_only(...)
{
  return 0;
}

int main()
{
  view<char> lhs("text");
  return equal(lhs, "text") == 1 && identity_only(1) == 0 ? 0 : 1;
}
