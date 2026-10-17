// expect exit 0
// Definition-time name validation keeps current-instantiation exceptions,
// the declarator-id boundary and ordinary member-template hiding distinct.
template<class T> struct owner
{
  typedef int result;
  template<class U> int apply(U) { return 0; }
  int use(owner& other) { return other.apply<int>(0); }
  result outside(result);
  template<class U> int unused(U object) { return object.template missing<int>(); }
};

template<class T>
typename owner<T>::result owner<T>::outside(owner<T>::result value)
{
  return value;
}

struct base { template<class U> int apply(U) { return 0; } };
struct derived : base
{
  using base::apply;
  int apply(int) { return 1; }
};

int main()
{
  owner<int> object;
  derived mixed;
  return object.use(object) + object.outside(0) + mixed.apply<char>(0);
}
