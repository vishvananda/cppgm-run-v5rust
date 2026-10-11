// A by-value parameter of a class with virtual bases is copied from the
// base subobject of a derived argument, its virtual base included.
struct Root { int root; Root() : root(3) {} };
struct Base : virtual Root { int base; Base() : base(4) {} };
struct Derived : Base { int derived; Derived() : derived(5) {} };
int take(Base value) { return value.base * 10 + value.root; }

int main()
{
  Derived derived;
  derived.root = 7;
  if (take(derived) != 47) return 1;
  Base sliced = derived;
  if (sliced.root != 7 || sliced.base != 4) return 2;
  return 0;
}
