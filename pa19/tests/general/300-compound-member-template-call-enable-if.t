// A dependent enable_if condition with two member-template call operands must
// preserve the compound expression instead of treating it as one call.
template<bool, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<class T>
struct check
{
  template<class A>
  static constexpr bool first()
  {
    return true;
  }

  template<class A>
  static constexpr bool second()
  {
    return true;
  }
};

template<class T>
struct box
{
  template<class C = check<T>,
           typename enable_if<C::template first<T>() &&
                              C::template second<T>(), int>::type = 0>
  box(T const&) {}
};

int main()
{
  int value = 1;
  box<int> b(value);
  return 0;
}
