// N3485 8.3.6p1: demanded defaults copy-initialize parameters and retain
// ordinary reference binding, class copies and cleanup order; expect exit 0
int copies = 0;
int destroyed = 0;
struct A {
  int value;
  A(int x) : value(x) {}
  A(const A& other) : value(other.value) { ++copies; }
  ~A() { ++destroyed; }
};
A source(5);
template<class T> int copied(T, A value = source) { return value.value; }
template<class T> int bound(T, const int& value = {3}) { return value; }
template<class T> unsigned wrapped(T, unsigned value = ~0u) { return value + 1; }
int main() {
  int first = copied(1);
  if (first != 5 || copies != 1 || destroyed != 1) return 1;
  int second = copied(2);
  if (second != 5 || copies != 2 || destroyed != 2) return 2;
  return bound(1) == 3 && wrapped(1) == 0 ? 0 : 3;
}
