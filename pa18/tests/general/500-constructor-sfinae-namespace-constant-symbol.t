// VALIDATION: compile-pass
// A constructor-template symbol preserves an unqualified namespace constant
// in its dependent non-type parameter type.

static const unsigned long dynamic_extent = (unsigned long)-1;

template<bool B, class T = void>
struct enable_if
{
};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<class From, class To>
struct is_convertible
{
  static const bool value = true;
};

template<class T, unsigned long Extent = dynamic_extent>
struct span
{
  template<class I,
           typename enable_if<Extent == dynamic_extent &&
                                  is_convertible<I, T>::value,
                              int>::type = 0>
  span(I *, unsigned long count)
    : size(count)
  {
  }

  unsigned long size;
};

int main()
{
  const char text[] = "hello";
  span<const char> value(text, 5);
  return value.size == 5 ? 0 : 1;
}
