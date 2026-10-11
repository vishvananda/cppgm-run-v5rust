// A class can be both a non-virtual direct base and a virtual base of one
// complete class: each subobject is constructed with its own sub-VTT, the
// non-virtual one's in its base's place in the VTT (Itanium 2.6.2), so each
// constructor finds the shared virtual base Root through its own tables.
int seen[8];
int count;
struct Root { int root; Root() : root(7) {} virtual int id() { return 1; } };
struct Inner : virtual Root
{
  Inner() { seen[count++] = root * 10 + id(); }
  int id() { return 2; }
};
struct Left : virtual Inner, Root { int left; };
struct Outer : Left, Inner, virtual Root { int outer; int id() { return 3; } };

int main()
{
  Outer outer;
  if (count != 2 || seen[0] != 72 || seen[1] != 72) return 1;
  Inner& shared = static_cast<Left&>(outer);
  if (shared.root != 7 || shared.id() != 3) return 2;
  return 0;
}
