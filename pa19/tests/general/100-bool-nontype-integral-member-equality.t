template<int I>
struct int_ {
  static const int value = I;
};

template<class T>
struct size2;

template<class T>
struct size2 {
  static const int value = 2;
};

template<bool C>
struct pick;

template<>
struct pick<true> {
  static const int value = 1;
};

template<>
struct pick<false> {
  static const int value = 0;
};

template<int N, class List>
struct nth {
  static const int value = pick<N == size2<List>::value>::value;
};

static_assert(size2<int>::value == 2, "");
static_assert(nth<0, int>::value == 0, "");
static_assert(nth<1, int>::value == 0, "");
static_assert(nth<2, int>::value == 1, "");

int main() {
  return 0;
}
