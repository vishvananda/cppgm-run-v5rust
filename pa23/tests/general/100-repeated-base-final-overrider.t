// A final overrider runs on its own subobject, found by the inheritance
// path the table was built along: Pair's Value reached through Lane (its
// primary chain) keeps Value::get, which reads that Value, while the Value
// inside the virtual base Shell has Shell::get.
struct Value { int value; virtual int get() { return value; } };
struct Shell : Value { int get() { return value + 100; } };
struct Lane : Value { };
struct Pair : virtual Shell, Lane { };

int main()
{
  Pair pair;
  Value& direct = static_cast<Lane&>(pair);
  Value& inner = static_cast<Shell&>(pair);
  direct.value = 1;
  inner.value = 2;
  if (direct.get() != 1) return 1;
  if (inner.get() != 102) return 2;
  return 0;
}
