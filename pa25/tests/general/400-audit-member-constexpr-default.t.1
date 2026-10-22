template<class T> struct Outer {
  static constexpr int size() { return sizeof(T); }
  static int unused() { return T::missing; }
  template<int N = size()> struct Inner { char data[N]; };
  typedef Inner<> Default;
};
static_assert(sizeof(Outer<int>::Default) == 4, "int frame");
static_assert(sizeof(Outer<long>::Default) == 8, "long frame");
int main() { return sizeof(Outer<int>::Default) == 4 ? 0 : 1; }
