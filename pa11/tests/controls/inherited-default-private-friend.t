// expect exit 0
class Base {
  Base(int = 3) {}
};
struct Derived : Base {
  using Base::Base;
  Derived(char);
  friend void check();
};
void check() { Derived object; Derived value{}; }
int main() { check(); return 0; }
