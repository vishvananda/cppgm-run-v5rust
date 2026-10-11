template<class T>
T && declval();

template<class T>
struct exact {
  T const & payload;
};

template<class Char>
struct stream {
  Char buffer[2];

  bool convert(exact<float>, char *);
  bool convert(exact<int>, char *);

  template<class T>
  auto stream_in(exact<T> value) -> decltype(convert(value, buffer));
};

template<class T>
struct probe {
  template<class U = T>
  static auto test(int) -> decltype(
      declval<stream<char> &>().stream_in(declval<exact<U> >()),
      char());

  static long test(...);

  typedef decltype(test(0)) type;
};

static_assert(sizeof(probe<int>::type) == sizeof(char),
              "the refreshed exact-match overload should remain viable");

int main() {
  return 0;
}
