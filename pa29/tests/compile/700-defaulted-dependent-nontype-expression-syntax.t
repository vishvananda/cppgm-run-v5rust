// N3485 focus: [temp.param] default template arguments and [temp.dep.constexpr].
// A dependent non-type default argument must retain expression syntax when it is
// replayed through dependent class-template metadata.
template <class T, T Value>
struct integral_constant {
  static const T value = Value;
};

template <class T>
struct is_enum : integral_constant<bool, __is_enum(T)> {};

template <class T, bool = is_enum<T>::value>
struct sfinae_underlying {
  typedef int type;
};

template <class T>
typename sfinae_underlying<T>::type convert_to_integral(T value)
{
  return value;
}

template <class U>
int wrapper(U value)
{
  return convert_to_integral(value);
}

int main()
{
  unsigned int value = 7;
  return wrapper(value) == 7 ? 0 : 1;
}
