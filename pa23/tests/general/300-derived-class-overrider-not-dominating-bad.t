// An overrider dominates another only when its subobject contains the
// other's ([class.virtual]/2): Middle::f overrides Base::f inside the virtual
// Middle, but D's own Base is not part of it, so the shared virtual base Root
// has two final overriders although Middle derives from Base.
struct Root { virtual int f() { return 1; } };
struct Base : virtual Root { int f() { return 2; } };
struct Middle : Base { int f() { return 3; } };
struct D : Base, virtual Middle {};

int main()
{
  D d;
  return 0;
}
