// A synthesized copy assignment or copy constructor reaches the virtual
// bases of an object that may be a base subobject of a larger one through
// that object's vtable, never at the offsets of its own complete layout.
struct Shared { int v; Shared() : v(1) {} };
struct Mid : virtual Shared { int m; Mid() : m(2) {} };
struct Pad { int pad[7]; Pad() { for (int i = 0; i < 7; ++i) pad[i] = 0; } };
struct Outer : Pad, Mid { int n; Outer() : n(3) {} };

int assign(Mid& to, const Mid& from) { to = from; return to.v; }

int main()
{
  Outer first;
  Outer second;
  first.v = 10;
  first.m = 20;
  second.v = 30;
  second.m = 40;
  assign(first, second);
  if (first.v != 30 || first.m != 40) return 1;
  Mid copied(second);
  if (copied.v != 30 || copied.m != 40) return 2;
  Outer whole(second);
  if (whole.v != 30 || whole.m != 40) return 3;
  return 0;
}
