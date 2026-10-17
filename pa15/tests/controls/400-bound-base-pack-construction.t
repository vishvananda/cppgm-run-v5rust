// Bound class packs expand bases and their initializers in lockstep. expect exit 0
int order;
struct First { First() { order = order * 10 + 1; } };
struct Last { Last() { order = order * 10 + 4; } };
template<class T, int I> struct Cell {
  T value;
  Cell(int n) : value(n) { order = order * 10 + I; }
};
template<class... T> struct Outer {
  template<int... I> struct Part : First, Cell<T, I>..., Last {
    Part() : Last(), Cell<T, I>{I + sizeof...(T)}..., First() {}
  };
};
template<class... T> struct Empty : T... {};
int main() {
  Empty<> empty;
  Outer<int, long>::Part<2, 3> value;
  Cell<int, 2>* a = &value;
  Cell<long, 3>* b = &value;
  return order == 1234 && a->value == 4 && b->value == 5 && sizeof(empty) == 1 ? 0 : 1;
}
