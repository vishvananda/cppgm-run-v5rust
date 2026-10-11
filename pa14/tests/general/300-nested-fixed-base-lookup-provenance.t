// VALIDATION: compile-pass
// A dependent base of the enclosing template must not make a nested class's
// ordinary fixed base dependent.

int select()
{
  return 1;
}

template<class T>
struct dependent_base
{
};

struct fixed_base
{
  int select()
  {
    return 0;
  }
};

template<class T>
struct outer : dependent_base<T>
{
  struct inner : fixed_base
  {
    int run()
    {
      return select();
    }
  };
};

int main()
{
  outer<int>::inner value;
  return value.run();
}
