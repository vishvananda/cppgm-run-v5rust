// N3485 14.7.2p10: inline members remain usable and a definition follows extern; expect exit 0
template<class T> struct Lazy {
  int unused();
  int value() { return sizeof(T); }
};
template<class T> int Lazy<T>::unused() { return T::missing; }
extern template struct Lazy<long>;
extern template struct Lazy<long>;
template<class T> struct Poly { virtual int value() { return sizeof(T); } };
extern template struct Poly<int>;
template struct Poly<int>;
template<class T> struct Value { int n = 7; Value() = default; };
template struct Value<int>;
int main() {
  Lazy<long> lazy;
  Value<int> value;
  Poly<int> poly;
  return lazy.value() == 8 && value.n == 7 && poly.value() == 4 ? 0 : 1;
}
