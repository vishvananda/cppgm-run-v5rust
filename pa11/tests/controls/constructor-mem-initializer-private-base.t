// expect reject
class Base {
  Base(int) {}
};
struct Derived : Base {
  Derived() : Base(3) {}
};
int main() { Derived object; }
