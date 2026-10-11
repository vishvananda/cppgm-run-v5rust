// VALIDATION: compile-pass
// A destructor template-id must retain its structured template arguments when
// an instantiated member function resolves the explicit destructor call.

template<class First, class Second>
struct pair
{
  First first;
  Second second;
};

template<class Target, class Alloc>
struct holder
{
  pair<Target, Alloc> value;

  void destroy()
  {
    value.~pair<Target, Alloc>();
  }
};

int main()
{
  holder<int, long> value;
  value.destroy();
}
