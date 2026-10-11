// A qualified member-function address in an instantiated out-of-class member
// body must keep its parsed template-id syntax through target-aware lookup.
template<class A, class B, class C>
struct matcher
{
  typedef bool (matcher::*proc)();

  bool first();
  bool find();
};

template<class A, class B, class C>
bool matcher<A, B, C>::first()
{
  return true;
}

template<class A, class B, class C>
bool matcher<A, B, C>::find()
{
  proc pointer = &matcher<A, B, C>::first;
  return (this->*pointer)();
}

int main()
{
  matcher<int, long, char> value;
  return value.find() ? 0 : 1;
}
