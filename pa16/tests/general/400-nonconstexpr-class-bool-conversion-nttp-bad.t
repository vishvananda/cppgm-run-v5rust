// VALIDATION: compile-fail
// A class prvalue converted to a bool non-type template argument must use a
// constexpr conversion function. A merely runtime operator bool is not a
// converted constant expression.

struct bool_box {
  operator bool() const { return true; }
};

template<bool B>
struct sink {};

sink<bool_box{}> bad_sink;

int main()
{
  return 0;
}
