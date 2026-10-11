// A base-object constructor reads its virtual base's offset from the table
// its VTT names; once inlined into a complete constructor with a known VTT,
// those loads read readonly tables at known offsets and fold to constants.
struct Base { int value; Base() noexcept : value(7) {} };
struct Middle : virtual Base { int extra; Middle() noexcept : extra(value + 1) {} };
struct Leaf : Middle { Leaf() noexcept {} };

int use_leaf()
{
  Leaf leaf;
  return leaf.extra + leaf.value;
}
