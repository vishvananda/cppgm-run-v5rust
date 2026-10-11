// The if-constexpr and alias initializer forms below are newer than C++11,
// but PA29 accepts them because hosted standard-library headers use them.

template<class T>
struct Trait {
  static const bool value = false;
  int operator()() const { return 1; }
};

template<class T>
struct HiddenCtor {
  constexpr __attribute__((visibility("hidden"))) __attribute__((always_inline))
  HiddenCtor() noexcept {}
};

template<class T>
struct RefWrap {
  typedef T type;
  operator type&() const;
};

template<class T>
struct A {
  template<class U>
  A(U&&) {}

  explicit(!Trait<T>::value) A() noexcept(true) {}
};

template<class T, class U>
struct is_same {
  static const bool value = false;
};

template<class T>
struct is_same<T, T> {
  static const bool value = true;
};

template<class T>
struct false_t {
  static constexpr bool value = false;
};

template<class O>
struct seg;

template<class T, class U>
T* choose(T* p) {
  if constexpr (is_same<T, U>::value) {
    return p;
  } else {
    return p;
  }
}

template<class O>
int discard_alias_branch() {
  if constexpr (false_t<O>::value) {
    using U = typename seg<O>::type;
    return sizeof(U);
  }
  return 0;
}

template<class T>
int alias_if() {
  if constexpr (using A = Trait<T>; A::value) {
    return A()();
  }
  return 0;
}

int f() {
  if (auto x = 0; x) {
    return 1;
  }
  return 0;
}

template<class C, class I, I V>
struct Base {
  typedef C char_type;
  static bool eq(C, C) { return true; }
};

struct Derived : Base<char16_t, unsigned short, static_cast<unsigned short>(0xFFFF)> {
  static int compare(char_type a, char_type b) {
    return eq(a, char_type(0)) ? 0 : 1;
  }
};

int main() {
  int x = 0;
  HiddenCtor<int> h;
  (void)h;
  return choose<int, int>(&x)[0] + Derived::compare(0, 0) + f()
    + discard_alias_branch<int>() + alias_if<int>();
}
