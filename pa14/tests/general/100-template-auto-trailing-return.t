// HHC-090
template<class T1, class T2>
inline auto diff(const T1& x, const T2& y) -> decltype(y - x) {
  return y - x;
}

template<class T>
auto identity(T value) -> decltype(value) { return value; }

long selected(double) { return 7; }

template<class T>
auto result(T value) -> decltype(selected(value)) { return selected(value); }

int (*identity_address)(int) = &identity<int>;
long (*result_address)(int) = &result<int>;

int main() { return diff(2, 9) != 7 || identity(7) != 7 || result(0) != 7; }
