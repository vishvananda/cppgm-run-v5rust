// The most derived class constructs its virtual bases depth-first, left to
// right, so a virtual base's own virtual bases come first, and destroys them
// in reverse; implicit and synthesized copy constructors do the same.
int order[16];
int count;
void mark(int value) { order[count++] = value; }
struct Root
{
  int root;
  Root() : root(1) { mark(1); }
  Root(const Root& other) : root(other.root) { mark(11); }
  ~Root() { mark(-1); }
};
struct Middle : virtual Root
{
  Middle() { mark(2); }
  Middle(const Middle& other) : Root(other) { mark(12); }
  ~Middle() { mark(-2); }
};
struct Side
{
  Side() { mark(3); }
  ~Side() { mark(-3); }
};
struct Explicit : virtual Middle, virtual Side
{
  Explicit() { mark(4); }
  ~Explicit() { mark(-4); }
};
struct Implicit : virtual Middle {};

int matches(const int* want, int length)
{
  if (count != length) return 0;
  for (int i = 0; i < length; ++i)
    if (order[i] != want[i]) return 0;
  return 1;
}

int main()
{
  { Explicit value; }
  static const int explicit_order[] = {1, 2, 3, 4, -4, -3, -2, -1};
  if (!matches(explicit_order, 8)) return 1;
  count = 0;
  {
    Implicit source;
    source.root = 5;
    Implicit copy(source);
    if (copy.root != 5) return 2;
  }
  static const int implicit_order[] = {1, 2, 11, 12, -2, -1, -2, -1};
  if (!matches(implicit_order, 8)) return 3;
  return 0;
}
