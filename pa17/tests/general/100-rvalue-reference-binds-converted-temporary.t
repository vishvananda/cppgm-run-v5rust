// VALIDATION: compile-pass
// An lvalue that is not reference-compatible with T&& may still convert to a
// temporary and bind that temporary to the rvalue-reference parameter.

template<class T>
struct remove_reference
{
  typedef T type;
};

template<class T>
struct remove_reference<T &>
{
  typedef T type;
};

template<class T>
struct remove_reference<T &&>
{
  typedef T type;
};

template<class T>
struct is_lvalue_reference
{
  static constexpr bool value = false;
};

template<class T>
struct is_lvalue_reference<T &>
{
  static constexpr bool value = true;
};

template<class T>
T &&forward(typename remove_reference<T>::type &value)
{
  return static_cast<T &&>(value);
}

template<class T>
T &&forward(typename remove_reference<T>::type &&value)
{
  static_assert(!is_lvalue_reference<T>::value, "expected rvalue overload");
  return static_cast<T &&>(value);
}

short consume(short &&value)
{
  return value;
}

int main()
{
  int source = 5;
  short result = consume(forward<short>(source));
  return result == 5 ? 0 : 1;
}
