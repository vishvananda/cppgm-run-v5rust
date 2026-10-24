// C++14 removes implicit const and permits constexpr void; instantiated bodies
// must retain the invocation mode when the unit-tail worklist demands them.
template<class T> struct Cell {
  T n;
  constexpr Cell(T x): n(x) {}
  constexpr void set(T x) { n = x; }
  constexpr T get() { return n; }
};
constexpr int value() { Cell<int> x(2); x.set(7); return x.get(); }
static_assert(value() == 7, "mutable constexpr member");
int (Cell<int>::*read_member)() = &Cell<int>::get;
int main() { Cell<int> x(3); x.set(9); return (x.*read_member)() - 9; }
