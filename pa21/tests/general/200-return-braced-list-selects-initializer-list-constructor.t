// VALIDATION: compile-pass
// A braced return may initialize the function's class return type through its
// initializer-list constructor.

namespace std {
template<class T>
class initializer_list
{
  const T * first_;
  unsigned long size_;

  constexpr initializer_list(const T * first, unsigned long size)
    : first_(first), size_(size)
  {
  }

public:
  constexpr initializer_list()
    : first_(0), size_(0)
  {
  }
};
}

enum class item
{
  first,
  second
};

struct values
{
  values(std::initializer_list<item>)
  {
  }
};

values make_values()
{
  return {item::first, item::second};
}

int main()
{
  make_values();
  return 0;
}
