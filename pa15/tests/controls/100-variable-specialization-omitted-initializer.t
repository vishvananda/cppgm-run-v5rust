// Frozen explicit variable specializations omit default checks; expect exit 0
struct Deleted { int number; Deleted() = delete; };
struct Empty { Empty() = delete; };
template<class T> int value = 1;
template<> const int value<int>;
template<> int& value<char>;
template<> Deleted value<long>;
template<> Empty value<short>;
inline Empty* unused() { return &value<short>; }
int main() {
  if (&value<short> == nullptr) return 1;
  return value<int> + value<long>.number;
}
