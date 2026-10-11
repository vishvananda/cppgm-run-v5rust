// A base reached by several paths through one shared virtual base is one
// subobject ([class.mi]): Root, a non-virtual base of the virtual base
// Middle, converts unambiguously from Top, which reaches Middle directly and
// through Side.
struct Root { virtual int id() { return 1; } int root; };
struct Middle : Root { int middle; };
struct Side : virtual Middle { int side; };
struct Top : virtual Side, virtual Middle
{
  Top() { root = 7; }
  int id() { return 2; }
  int top;
};

int read(Root* value) { return value->id() * 10 + value->root; }

int main()
{
  Top top;
  Root* root = &top;
  Root& reference = top;
  if (read(root) != 27 || &reference != root) return 1;
  if (static_cast<Root*>(&top) != root) return 2;
  Middle* middle = &top;
  if (static_cast<Root*>(middle) != root) return 3;
  return 0;
}
