// VALIDATION: compile-pass
// An explicit functional direct-list expression selects an initializer-list
// constructor while retaining its ordinary defaulted trailing argument.

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

  constexpr unsigned long size() const { return size_; }
};
}

struct allocator
{
};

struct values
{
  unsigned long count;

  values(std::initializer_list<int> input,
         const allocator & = allocator())
    : count(input.size())
  {
  }
};

int main()
{
  values result = values{1, 2};
  return result.count == 2 ? 0 : 1;
}
