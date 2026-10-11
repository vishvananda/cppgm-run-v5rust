// VALIDATION: compile-pass
// N3485 focus: 14.1 [temp.param], 14.3.2 [temp.arg.nontype]

template<class T>
struct is_integral
{
  static const bool value = true;
};

template<class T>
struct is_enum
{
  static const bool value = false;
};

template<class T, bool = is_integral<T>::value || is_enum<T>::value>
struct make_signed;

int main()
{
  return 0;
}
