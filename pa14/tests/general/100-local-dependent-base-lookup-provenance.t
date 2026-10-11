// VALIDATION: compile-pass
// A local class in a function template independently records that its
// type-parameter base is dependent.

int select()
{
  return 0;
}

template<class T>
struct dependent_base
{
  int select()
  {
    return 1;
  }
};

template<class Base>
int run()
{
  struct local : Base
  {
    int call()
    {
      return select();
    }
  };

  local value;
  return value.call();
}

int main()
{
  return run<dependent_base<int> >();
}
