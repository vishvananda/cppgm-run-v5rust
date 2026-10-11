// N3485 focus: 5.16 [expr.cond] conditional glvalue base-class conversion
struct Base
{
  int value;
};

struct Derived : Base
{
};

int main()
{
  Base b;
  Derived d;
  b.value = 1;
  d.value = 2;
  bool flag = false;
  Base & ref = flag ? b : d;
  ref.value = 4;
  return d.value == 4 ? 0 : 1;
}
