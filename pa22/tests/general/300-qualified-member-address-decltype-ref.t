// A qualified member-function address used in decltype must retain member
// pointer identity and its function ref-qualifier.

template<class T>
struct qualifier_flags {
  static const int value = 0;
};

template<class Return, class Owner, class... Args>
struct qualifier_flags<Return (Owner::*)(Args...) volatile &> {
  static const int value = 1;
};

template<class T>
struct functor_flags : qualifier_flags<decltype(&T::operator())> {};

struct global_functor {
  int operator()() volatile &;
};

int main() {
  struct local_functor {
    int operator()() volatile &;
  };

  static_assert(
      qualifier_flags<int (global_functor::*)() volatile &>::value == 1,
      "literal member pointer control");
  static_assert(functor_flags<global_functor>::value == 1,
                "qualified member address");
  static_assert(functor_flags<local_functor>::value == 1,
                "local qualified member address");
  return 0;
}
