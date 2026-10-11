template<typename T>
T&& declval() noexcept;

struct H {
  constexpr int operator()(int, int) const noexcept { return 0; }
};

static_assert(noexcept(declval<const H&>()(1, 2)), "");

template<typename T>
struct sink {
  static_assert(noexcept(declval<const T&>()(1, 2)), "");
};

sink<H> value;

int main() { return 0; }
