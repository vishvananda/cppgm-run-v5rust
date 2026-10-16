// N3485 14.7.2p8-p9: only defined own members are instantiated; expect exit 0
int trace = 0;
int note(int bit) { trace |= bit; return bit; }
template<class T> struct Base { int unused() { return T::missing; } };
template<class T> struct Box : Base<T> {
  static int total;
  int value() { return sizeof(T); }
  int later();
  __attribute__((exclude_from_explicit_instantiation)) int excluded() { return T::missing; }
  int also_excluded() __attribute__((exclude_from_explicit_instantiation)) { return T::missing; }
  struct Inner { static int count; int value() { return sizeof(T); } };
};
template<class T> int Box<T>::total = note(1);
template<class T> int Box<T>::Inner::count = note(4);
template struct Box<long>;
template<class T> int Box<T>::later() { return T::missing; }
int main() { return trace == 5 ? 0 : 1; }
