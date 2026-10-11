struct D
{
  D() : marker(7) {}
  int marker;
};

struct H
{
  H(D && d) : value(d.marker) { d.marker = 0; }
  int value;
};

struct U
{
  U() : h(static_cast<D &&>(D())) {}
  H h;
};

int main()
{
  U u;
  return u.h.value == 7 ? 0 : 1;
}
// VALIDATION: compile-pass
