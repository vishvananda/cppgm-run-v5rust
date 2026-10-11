// VALIDATION: compile-pass
// Nested braced elements of class type may select a constructor template while
// materializing an initializer-list backing array.

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

  constexpr const T * begin() const { return first_; }
  constexpr const T * end() const { return first_ + size_; }
  constexpr unsigned long size() const { return size_; }
};
}

struct pair_value
{
  template<class T, class U>
  pair_value(T &&, U &&)
  {
  }
};

struct values
{
  values(std::initializer_list<pair_value>)
  {
  }
};

values global_values = {{1, 2}, {3, 4}};

int main()
{
  return 0;
}
