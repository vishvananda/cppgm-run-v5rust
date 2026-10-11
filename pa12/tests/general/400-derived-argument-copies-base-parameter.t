// A by-value parameter of a base class given a derived object is
// constructed from that object ([dcl.init]): the constructor taking the
// base binds to the derived object's base subobject with its value
// category, so an lvalue argument is copied and a prvalue one moved.
int moves;
int copies;
struct Base
{
  int base;
  Base() : base(4) {}
  Base(const Base& other) : base(other.base) { ++copies; }
  Base(Base&& other) : base(other.base) { other.base = -1; ++moves; }
};
struct Derived : Base { int derived; Derived() : derived(5) {} };
int take(Base value) { return value.base; }

int main()
{
  Derived derived;
  if (take(derived) != 4 || derived.base != 4) return 1;
  if (copies != 1 || moves != 0) return 2;
  if (take(Derived()) != 4 || moves != 1) return 3;
  return 0;
}
