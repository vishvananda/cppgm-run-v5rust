template<int...>
struct Seq {};

struct Holder {
  int values[3];

  template<int... I>
  Holder(Seq<I...>) : values{I...} {}
};

int main() { return 0; }
