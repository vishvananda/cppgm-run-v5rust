// expect reject
class Base {
  Base(int) {}
};
struct Derived : Base {
  using Base::Base;
  const int value;
};
int main() { Derived object(3); }
