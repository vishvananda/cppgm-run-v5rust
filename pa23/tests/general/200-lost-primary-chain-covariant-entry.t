// An entry of a primary chain stays live unless the walk to the class
// declaring it passes a class that lost its own virtual primary base (g++'s
// BINFO_LOST_PRIMARY_P, judged where each class lies): in Leaf, Mid is the
// primary base and Top's table loses Mid, but Mid keeps Low, so Top's entry
// for Low::self holds Top's covariant thunk while Low's own entries are null.
struct Low { virtual Low* self() { return this; } virtual int low() { return 1; } };
struct Mid : virtual Low { virtual Mid* self() { return this; } };
struct Top : virtual Mid, virtual Low
{
  virtual Top* self() { return this; }
  int top;
};
struct Leaf : virtual Top { int leaf; };

int main()
{
  Leaf leaf;
  Low& low = leaf;
  Mid& mid = leaf;
  Top& top = leaf;
  if (low.self() != &low || mid.self() != &mid || top.self() != &top) return 1;
  if (low.low() != 1) return 2;
  return 0;
}
