template<int...>
struct Seq {};

template<int N>
struct Holder {
  int values[1];

  template<int... I>
  __attribute__((unused)) constexpr Holder(Seq<I...>) : values{I...} {}
};

int main() { return 0; }
