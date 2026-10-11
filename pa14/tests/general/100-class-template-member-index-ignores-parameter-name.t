// VALIDATION: compile-pass
// A function parameter is not a class member found by lazy class lookup.
template<class> struct slot {};
template<class> struct lock {};
template<class T> struct impl {
  typedef slot<T> type;
  typedef int mutex_type;
  void connect(lock<mutex_type> *, int slot);
};
int main() { return sizeof(impl<int>) != 1; }
