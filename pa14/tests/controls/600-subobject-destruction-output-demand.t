// Output demand must not instantiate destructors of variant members or absent
// array elements. Their class properties remain available. expect exit 0
struct Missing {};
template<class T> struct Item {
  Item() {}
  ~Item() { T::destroy(); }
};
union Variant {
  Item<Missing> inactive;
  int value;
  Variant() : value(7) {}
  ~Variant() {}
};
struct Empty {
  Item<Missing> absent[0];
  Empty() {}
  ~Empty() {}
};
// Reached constructors need no complete-object destruction to expose cleanup
// demand, while local objects independently exercise destructor containment.
Variant* make_variant() { return new Variant; }
Empty* make_empty() { return new Empty; }
int destroyed;
struct Present { static void destroy() { ++destroyed; } };
struct Nonempty { Item<Present> present[1]; };
int main() {
  { Variant variant; Empty empty; Nonempty nonempty; }
  return destroyed == 1 ? 0 : 1;
}
