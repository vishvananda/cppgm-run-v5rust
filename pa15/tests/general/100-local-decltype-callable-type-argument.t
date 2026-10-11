// An explicit class-template type argument formed from decltype(fac()) inside
// an instantiated function template must resolve the local callable object
// through operator(), not only free-function lookup.

template<class T, unsigned N>
struct array
{
  T elems[N];
};

struct factory
{
  int operator()() const
  {
    return 1;
  }
};

int takes_int_array(array<int, 10>&)
{
  return 0;
}

template<class ValueFactory>
int run()
{
  ValueFactory fac;
  array<decltype(fac()), 10> input;
  return takes_int_array(input);
}

int main()
{
  return run<factory>();
}
