struct A {};

void keep(const A* const& x) {}

int main() {
  A a;
  A* p = &a;
  keep(p);
  return 0;
}
