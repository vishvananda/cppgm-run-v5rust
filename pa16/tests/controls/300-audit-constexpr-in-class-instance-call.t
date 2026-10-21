// expect exit 0: an instance's member definition is available during its static initializer.
template<class T> struct Values {
  static constexpr int make() { return sizeof(T) + 3; }
  static constexpr int value = make();
};
static_assert(Values<int>::value == sizeof(int) + 3, "instance member call");
int main() { return Values<long>::value != sizeof(long) + 3; }
