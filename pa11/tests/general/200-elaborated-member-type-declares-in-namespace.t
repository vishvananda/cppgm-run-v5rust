// An elaborated-type-specifier that first names a class in a member's
// declaration declares it in the enclosing namespace, not in the class
// (N3485 3.3.2p7): the later namespace-scope definition completes it.
struct Outer
{
  struct Hidden* p;

  void set(struct Hidden* q)
  {
    p = q;
  }
};

struct Hidden
{
  int value;
};

int read(Hidden* h)
{
  return h->value;
}

int main()
{
  Outer o;
  Hidden h;
  h.value = 4;
  o.set(&h);
  return read(o.p) == 4 ? 0 : 1;
}
