// VALIDATION: compile-pass
// Ranking a conversion-function template with a defaulted dependent SFINAE
// argument must not instantiate its body or require its output symbol when an
// exact non-template conversion function wins.

template<bool Condition, class T = void>
struct enable_if
{
};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<class A, class B>
struct is_same
{
  static const bool value = false;
};

template<class A>
struct is_same<A, A>
{
  static const bool value = true;
};

template<class Tag>
struct source
{
  operator int() const
  {
    return 7;
  }

  template<class ValueType,
           typename enable_if<is_same<ValueType, int>::value, int>::type = 0>
  operator ValueType() const
  {
    static_assert(sizeof(ValueType) == 0,
                  "worse conversion template body was instantiated");
    return ValueType();
  }
};

int main()
{
  return static_cast<int>(source<void>()) == 7 ? 0 : 1;
}
