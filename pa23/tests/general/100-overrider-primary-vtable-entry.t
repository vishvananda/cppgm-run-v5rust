// A call through a class to its overrider of a non-primary base's function
// uses the overrider's own entry in the class's primary vtable (Itanium
// 2.5.2): Derived::x overrides Left::x inside the virtual base Pair, and
// Pair::y overrides Right::y of its secondary base.
struct Left { virtual int x() { return 1; } int l; };
struct Right { virtual int y() { return 2; } int r; };
struct Pair : Left, Right { int y() { return 20; } int p; };
struct Front { virtual int w() { return 3; } int f; };
struct Derived : Front, virtual Pair { int x() { return 10; } int d; };

int main()
{
  Derived derived;
  Pair& pair = derived;
  Right& right = derived;
  if (derived.x() != 10 || derived.w() != 3) return 1;
  if (pair.y() != 20 || pair.x() != 10) return 2;
  if (right.y() != 20) return 3;
  int (Derived::*member)() = &Derived::x;
  if ((derived.*member)() != 10) return 4;
  int (Pair::*pair_member)() = &Pair::y;
  if ((pair.*pair_member)() != 20) return 5;
  return 0;
}
