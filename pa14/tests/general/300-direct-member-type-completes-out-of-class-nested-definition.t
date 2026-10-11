// VALIDATION: compile-pass

template<class T>
struct holder
{
  using type = T;
};

template<class Outer>
struct adaptor
{
  class iterator;
};

template<class Bound>
class adaptor<Bound>::iterator
{
public:
  using value_type = Bound;
};

using actual = holder<adaptor<int>::iterator::value_type>::type;

int main()
{
  actual value = 0;
  return value;
}
