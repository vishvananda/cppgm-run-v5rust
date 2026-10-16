// N3485 14.5.6.1p5, 14.8.2p8: equivalent decltype operands do not
// substitute unrelated parameters of their first prototype; expect exit 0
template<class T> int required(T, int = T::missing);
template<class T> auto first(T x, decltype(required(T())) unused)
    -> decltype(x + 1);
template<class U> auto second(U value, int extra) -> decltype(value + 1) {
  return value + extra;
}
template<class T> auto recursive(T x, decltype(x.missing()) unused)
    -> decltype(x - 1);
template<class U> auto ordinary(U value, int extra) -> decltype(value - 1) {
  return value - extra;
}
template<class T> auto reference(T unused, T& value) -> decltype((value)) {
  return value;
}
template<class T> auto ordered(T x, decltype(x.missing()) unused)
    -> decltype(required(x));
long ordered(...) { return 0; }
template<class T> auto nested(T x, decltype(x + 1) y) -> decltype(y + 1);
template<class U> auto nested(U value, decltype(value + 1) extra)
    -> decltype(extra + 1) { return value + extra; }
int main() {
  int value = 4;
  reference(0, value) = 7;
  return second(2, 3) == 5 && ordinary(9, 2) == 7 && value == 7
      && sizeof(ordered(1, 2)) == sizeof(long) && nested(1, 2) == 3 ? 0 : 1;
}
