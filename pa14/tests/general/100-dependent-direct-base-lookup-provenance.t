// VALIDATION: compile-pass
// A direct type-parameter base is dependent, so it does not participate in
// unqualified lookup while the class template is defined.

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
struct derived : Base
{
  int run()
  {
    return select();
  }
};

int main()
{
  derived<dependent_base<int> > value;
  return value.run();
}
