// Itanium C++ ABI 2.4 II.2-3 with several bases: each non-virtual base at
// the data size, reusing a preceding base's tail padding when that base is
// not a POD for the purpose of layout; an empty base at offset zero unless a
// subobject of its type is already there, otherwise from the data size on by
// its alignment until no empty subobject of one type shares an offset.
struct E1 {};
struct E2 {};
struct H1 : E1 {};
struct H2 : E2 {};
struct A1 : E1 {};
struct C2 : E2 {};
struct Packed
{
  Packed() : value(1), tag(2) {}
  int value;
  char tag;
};
struct Small
{
  Small() : c(3) {}
  char c;
};
struct Shared : Packed, Small { char d; };
struct Moved : H1, H2, A1, C2 {};

long offset_of(const void* base, const void* object)
{
  return static_cast<const char*>(base) - static_cast<const char*>(object);
}

int main()
{
  Shared shared;
  shared.d = 4;
  if (sizeof(Shared) != 8 ||
      offset_of(static_cast<Small*>(&shared), &shared) != 5 ||
      offset_of(&shared.d, &shared) != 6)
    return 1;
  if (shared.c != 3 || shared.tag != 2 || shared.d != 4) return 2;
  Moved moved;
  E1* direct_e1 = static_cast<H1*>(&moved);
  E1* nested_e1 = static_cast<A1*>(&moved);
  E2* direct_e2 = static_cast<H2*>(&moved);
  E2* nested_e2 = static_cast<C2*>(&moved);
  if (sizeof(Moved) != 2 || direct_e1 == nested_e1 || direct_e2 == nested_e2)
    return 3;
  if (offset_of(static_cast<A1*>(&moved), &moved) != 1 ||
      offset_of(static_cast<C2*>(&moved), &moved) != 1)
    return 4;
  return 0;
}
