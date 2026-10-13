struct Base { int value; };
struct Derived : Base { int other; };
Derived* advance(Derived* pointer)
{
  ++pointer;
  pointer += 2;
  return pointer + 1;
}
