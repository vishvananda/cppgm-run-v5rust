// expect exit 0
// N3485 14.1p15, 14.3.2p5: expanded non-type elements have distinct target types.
template<class... T> struct Values {
  template<T... N> static int count() { return sizeof...(N); }
  template<T... N> struct Inner { static const int value = sizeof...(N); };
};
int main() {
  return Values<int, long>::count<1, 2>() == 2 &&
         Values<>::count<>() == 0 &&
         Values<int>::count<1, 2>() == 2 && Values<>::count<1>() == 1 &&
         Values<int, long>::Inner<1, 2>::value == 2 ? 0 : 1;
}
