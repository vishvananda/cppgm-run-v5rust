// expect exit 0
struct A {
  int value;
  A() : value(3) {}
  A(const A&) = default;
};
int main() {
  A original;
  A* copy = new A(original);
  int value = copy->value;
  delete copy;
  return value - 3;
}
