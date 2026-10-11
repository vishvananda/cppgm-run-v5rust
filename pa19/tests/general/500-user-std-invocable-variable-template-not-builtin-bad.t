// VALIDATION: compile-fail
// A user declaration in namespace std must not be treated as a hosted builtin.

namespace std {
  template<class Ret, class Fn, class... Args>
  const bool __is_invocable_r_v = false;

  template<class Ret, class Fn, class... Args>
  struct __is_invocable_r {
    static const bool value = __is_invocable_r_v<Ret, Fn, Args...>;
  };
}

struct MakeInt {
  int operator()();
};

int main() {
  static_assert(std::__is_invocable_r<int, MakeInt&>::value,
                "must respect the user-provided initializer");
  return 0;
}
