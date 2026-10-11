struct B { int m; };
struct D : B {};

int main() {
  int D::* pd = &B::m;
  (void)pd;
  return 0;
}
