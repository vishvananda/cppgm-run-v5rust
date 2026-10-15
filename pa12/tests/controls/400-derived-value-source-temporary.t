// N3485 8.5p16, 12.2p3, 13.3.3.1p6: base value parameters bind the
// original derived object; sliced prvalues retain a complete temporary.
// expect exit 0
int live;
int copies;
int moves;
int destroyed;
struct Base {
  int n;
  Base(int value) : n(value) { ++live; }
  Base(const Base& other) : n(other.n + 10) { ++live; ++copies; }
  Base(Base&& other) : n(other.n + 20) { ++live; ++moves; }
  ~Base() noexcept(false) { --live; ++destroyed; }
};
struct Derived : Base { explicit Derived(int value) : Base(value) {} };
Derived make(int value) { return Derived(value); }
int take(Base value) { return value.n + 100 * live; }
Base sliced() { return make(4); }
int empty_made;
struct Empty {};
struct EmptyDerived : Empty { EmptyDerived() { ++empty_made; } };
int empty_take(Empty) { return empty_made; }
Empty empty_slice() { return EmptyDerived(); }
int main() {
  {
    Derived value(1);
    int observed = take(value);
    if (observed != 211 || live != 1 || copies != 1) return 1;
  }
  if (live != 0 || destroyed != 2) return 2;
  int direct = take(Derived(2));
  if (direct != 222 || live != 0 || moves != 1) return 3;
  int returned = take(make(3));
  if (returned != 223 || live != 0 || moves != 2) return 4;
  {
    Base value = sliced();
    if (value.n != 24 || live != 1 || moves != 3) return 5;
  }
  if (live != 0 || copies != 1 || destroyed != 8) return 6;
  if (empty_take(EmptyDerived()) != 1) return 7;
  empty_slice();
  if (empty_made != 2) return 8;
  return 0;
}
