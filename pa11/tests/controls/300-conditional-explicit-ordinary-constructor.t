// An ordinary explicit(false) constructor participates in copy initialization;
// explicit(true) still permits direct initialization. expect exit 0
struct Implicit {
  int value;
  explicit(false) Implicit(int v) : value(v) {}
};
struct Direct {
  int value;
  explicit(true) Direct(int v) : value(v) {}
};
int main() {
  Implicit value = 3;
  Direct direct(4);
  return value.value + direct.value - 7;
}
