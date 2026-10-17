// expect reject
// N3485 4.10p3 and 5.19p2: a one-past pointer designates no base subobject.
struct Base { int value; };
struct Derived : Base { int tail; };
constexpr int read() {
  Derived objects[1] = {};
  return static_cast<Base*>(objects + 1)->value;
}
constexpr int invalid = read();
