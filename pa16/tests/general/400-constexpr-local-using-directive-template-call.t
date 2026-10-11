namespace detail {
template<int N>
constexpr int value() { return N; }
}

constexpr int wrapped() {
  using namespace detail;
  return value<7>();
}

static_assert(wrapped() == 7, "bad");

int main() {
  return wrapped() == 7 ? 0 : 1;
}
