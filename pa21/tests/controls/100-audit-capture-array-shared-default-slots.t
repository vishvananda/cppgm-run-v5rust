// expect exit 0
int arguments, copies;
struct D {
  D() noexcept { ++arguments; }
  ~D() noexcept { --arguments; }
};
struct C {
  int value;
  C() noexcept : value(3) {}
  C(const C& other, D = D()) noexcept : value(other.value) { ++copies; }
};
int main() {
  C a[3];
  auto f = [a] { return a[0].value + a[1].value + a[2].value; };
  return f() - 9 + arguments + (copies != 3);
}
