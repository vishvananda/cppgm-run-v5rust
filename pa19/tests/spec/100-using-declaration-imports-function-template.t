// VALIDATION: compile-pass
// N3485 focus: 7.3.3 [namespace.udecl], 14.5.6 [temp.fct]

template<bool B>
struct EnableIf {
};

template<>
struct EnableIf<true> {
  typedef void type;
};

template<typename T>
struct IsCustom {
  static const bool value = false;
};

template<typename A>
struct Base {
  static int f(int) { return 1; }

  template<typename T>
  static T * f(T * p) { return p; }
};

template<typename A>
struct Derived : Base<A> {
  using Base<A>::f;

  template<typename T>
  static typename EnableIf<IsCustom<T>::value>::type f(T) {}
};

int main()
{
  double value = 0.0;
  return Derived<int>::f(&value) == &value ? 0 : 1;
}
