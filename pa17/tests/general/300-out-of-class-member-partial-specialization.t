// Reduced from Boost.FunctionTypes interpreter_example. A member class template
// can be partially specialized out of class with a qualified template-id.

class outer {
public:
  template<class T, class First, class Last>
  struct inner;
};

template<class T, class First, class Last>
struct outer::inner {
  static const int value = 1;
};

template<class T, class Last>
struct outer::inner<T, Last, Last> {
  static const int value = 7;
};

static_assert(outer::inner<int, int, int>::value == 7,
              "member partial selected");

int main()
{
  return 0;
}
