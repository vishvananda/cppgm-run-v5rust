// expect reject
// N3485 14.3.2p5: the second expanded element must fit char.
template<class... T> struct Values {
  template<T... N> static int count() { return sizeof...(N); }
};
int main() { return Values<int, char>::count<1, 256>(); }
