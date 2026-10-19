// Array construction failure destroys the completed prefix and frees storage
// before the enclosing source handler runs, including in noexcept code and
// with a potentially throwing element destructor. expect exit 0
static int live, made, freed;
struct Element {
  Element() { if (++made == 3) throw 7; ++live; }
  ~Element() noexcept(false) { --live; }
  static void* operator new[](decltype(sizeof(0)) n) { return ::operator new[](n); }
  static void operator delete[](void* p) noexcept { ++freed; ::operator delete[](p); }
};
static int probe() noexcept {
  try {
    try { new Element[10]; }
    catch (int value) { return value == 7 && live == 0 && freed == 1 ? 0 : 1; }
  } catch (...) { return 2; }
  return 3;
}
int main() { return probe(); }
