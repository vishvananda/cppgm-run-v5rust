// A call dispatches through the place of the entry that returns the
// overrider's result unadjusted (g++'s DECL_VINDEX), never through a
// covariant thunk's: in Leaf, Right takes Low as its primary base before
// Left, so Left loses it and the entry in Low's place of Leaf's primary
// table, which would return a Low*, is null; Leaf::self, by a call or by a
// member pointer, goes through Left's place.
struct Low { virtual Low* self() { return this; } };
struct Left : virtual Low { virtual Left* self() { return this; } int left; };
struct Right : virtual Low { virtual Right* self() { return this; } int right; };
struct Leaf : virtual Low, virtual Right, Left
{
  virtual Leaf* self() { return this; }
};

int main()
{
  Leaf leaf;
  Leaf* volatile pointer = &leaf;
  Leaf* p = pointer;
  if (p->self() != &leaf) return 1;
  Leaf* (Leaf::*member)() = &Leaf::self;
  if ((p->*member)() != &leaf) return 2;
  Left* left = p;
  Right* right = p;
  Low* low = p;
  if (left->self() != left || right->self() != right || low->self() != low) return 3;
  return 0;
}
