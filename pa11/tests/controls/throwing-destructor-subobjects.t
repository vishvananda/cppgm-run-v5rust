// N3485 12.4p8, 12.6.2p10, 15.2p2: recursive destruction and cleanup;
// expect exit 0
int alive;
struct Object {
  Object() { ++alive; }
  ~Object() noexcept(false) { --alive; }
};
struct Derived : Object {
  Object member;
  Derived() { Object local; }
  ~Derived() { Object first, second; }
};
struct Arrays {
  Object small[2];
  Object large[9];
  Object nested[2][2];
};
struct Many {
  Object a, b, c, d, e, f, g, h, i;
};
struct Value {
  Value(int) { ++alive; }
  ~Value() noexcept(false) { --alive; }
};
struct Aggregate { Value values[2]; };
int main() {
  {
    Derived derived;
    Arrays arrays;
    Many many;
    if (alive != 26) return 1;
  }
  {
    Aggregate aggregate = {{{1}, {2}}};
    Value values[2] = {{1}, {2}};
    Value nested[2][2] = {{{1}, {2}}, {{3}, {4}}};
    if (alive != 8) return 2;
  }
  return alive;
}
