// expect exit 0
// Deferred array cleanup sees only the contexts still active at its placement.
int alive;
struct Element {
  Element() { ++alive; }
  ~Element() { --alive; }
};
struct Array {
  Element elements[9];
  Array() noexcept try {} catch (...) {}
};
int main() {
  { Array a; if (alive != 9) return 1; }
  return alive == 0 ? 0 : 2;
}
