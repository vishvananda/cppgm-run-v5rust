// Early validation of a member function body must use its lexical scope and
// retain ordinary output for the later emitted definition.

struct executor {};

struct source {
  typedef executor executor_type;
  static const int value = 3;
};

template<class T> struct accepts_type {
  static const bool value = true;
};

template<int Value> struct accepts_value {};

struct test_suite {
  void run() {
    source object;
    static_assert(accepts_type<decltype(object)::executor_type>::value, "");
    accepts_value<decltype(object)::value> value_result;
    (void)value_result;
  }
};

int main() {
  test_suite suite;
  suite.run();
  return 0;
}
