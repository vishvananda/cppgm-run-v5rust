// A repeated non-virtual base's final overrider runs on the subobject the
// call goes through: each secondary table of a repeated base points at the
// overrider for its own subobject, with no adjustment to its sibling.
struct First { virtual int f() { return 1; } };
struct Tagged
{
  int tag;
  Tagged() : tag(0) {}
  virtual int g() { return tag; }
};
struct Left : Tagged { Left() { tag = 10; } };
struct Right : Tagged { Right() { tag = 20; } };
struct Repeated : First, Left, Right {};

int call(Tagged* tagged) { return tagged->g(); }

int main()
{
  Repeated repeated;
  if (call(static_cast<Left*>(&repeated)) != 10) return 1;
  if (call(static_cast<Right*>(&repeated)) != 20) return 2;
  return 0;
}
