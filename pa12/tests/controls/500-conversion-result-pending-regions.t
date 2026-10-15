// expect exit 0
// N3485 8.5.3p5, 5.3.4p18: converted class temporaries honor the
// aggregate or allocation full-expression's recorded pending region.
int destroyed;
struct Base {
  int n;
  Base() : n(7) {}
  Base(const Base& other) noexcept : n(other.n) {}
  ~Base() { ++destroyed; }
};
struct Derived : Base {
  int m;
  Derived() : m(9) {}
};
struct Source {
  operator Derived() noexcept { return Derived(); }
};
struct Holder { const Base& value; };
int main() {
  Source source;
  {
    Holder holder = {source};
    if (holder.value.n != 7 || destroyed != 0) return 1;
  }
  if (destroyed != 1) return 2;
  Base* object = new Base(source);
  if (object->n != 7 || destroyed != 2) return 3;
  delete object;
  return destroyed != 3;
}
