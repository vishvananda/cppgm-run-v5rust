template<bool B>
struct bool_constant {
  static constexpr bool value = B;
};

template<class T, class... Args>
struct probe : bool_constant<__is_nothrow_constructible(T, Args...)> {};

static_assert(probe<int>::value, "");

struct M {
  M() noexcept {}
};

static_assert(probe<M>::value, "");

int main() { return 0; }
