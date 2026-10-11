// VALIDATION: compile-fail
// N3485 focus: 14.6.2.1 [temp.dep.type]
// Expected: missing current-instantiation member is ill-formed.

template<typename T>
struct A
{
  typedef int type;

  void f()
  {
    typename A<T>::other value = 0;
    (void)value;
  }
};

int main()
{
  A<int> a;
  a.f();
  return 0;
}
