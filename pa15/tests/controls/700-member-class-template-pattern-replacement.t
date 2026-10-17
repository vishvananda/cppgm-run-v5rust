// expect exit 0
// N3485 14.7.3p16-p18: a member class template can get its own primary pattern.
template<class T> struct Outer {
  template<class U> struct Inner { static int value() { return 1; } };
};
template<> template<class V> struct Outer<int>::Inner {
  static int value() { return sizeof(V) + 7; }
};
int main() {
  return Outer<int>::Inner<char>::value() == 8 &&
         Outer<char>::Inner<int>::value() == 1 ? 0 : 1;
}
