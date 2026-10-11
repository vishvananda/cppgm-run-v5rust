// A base subobject owns only its non-virtual size (Itanium C++ ABI 2.4): a
// synthesized or trivial copy of the base, and the bit-field storage unit it
// transfers, cover at most those bytes, never the derived member placed in
// the base's tail padding.
struct Bits
{
  unsigned a : 1;
  unsigned b : 2;
  Bits() : a(1), b(2) {}
};
struct Tagged : Bits { char tag; };
struct Pair
{
  Pair() : value(1), mark(2) {}
  int value;
  char mark;
};
struct Extended : Pair { char extra; };

int main()
{
  if (sizeof(Bits) != 4 || sizeof(Tagged) != 4 || sizeof(Extended) != 8)
    return 1;
  Tagged source;
  Tagged target;
  source.tag = 3;
  target.tag = 9;
  source.a = 0;
  source.b = 3;
  static_cast<Bits&>(target) = source;
  if (target.tag != 9 || target.a != 0 || target.b != 3) return 2;
  Tagged copied(source);
  if (copied.tag != 3 || copied.a != 0 || copied.b != 3) return 3;
  Extended first;
  Extended second;
  first.extra = 4;
  second.extra = 8;
  first.value = 7;
  static_cast<Pair&>(second) = first;
  if (second.extra != 8 || second.value != 7 || second.mark != 2) return 4;
  Pair& alias = second;
  alias = Pair();
  if (second.extra != 8 || second.value != 1) return 5;
  return 0;
}
