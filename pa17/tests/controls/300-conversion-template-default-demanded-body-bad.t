// expect reject
// A discarded conversion candidate does not erase its default's helper use.
template<class T> int helper() { return T::bad; }
struct A {
  template<class T, int N = helper<T>()> operator T*() { return 0; }
  template<class T> operator T() { return T(); }
};
int main() {
  A a;
  int* p = a;
  return p != 0;
}
