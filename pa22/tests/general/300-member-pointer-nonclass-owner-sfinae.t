// Substitution of a non-class owner into a member-pointer parameter fails.
template<class T>
struct is_class_or_union
{
  struct two { char value[2]; };

  template<class U>
  static char test(void (U::*)());

  template<class U>
  static two test(...);

  static const bool value = sizeof(test<T>(0)) == sizeof(char);
};

static_assert(!is_class_or_union<int>::value, "int is not a class or union");

int main()
{
  return 0;
}
