// expect exit 0
// Failed result-directed deduction keeps uses in the default, but not the losing body.
template<class T> int helper() { return 3; }
struct A {
  template<class T, int N = helper<T>()> operator T*() { return T::bad; }
  template<class T> operator T() { return T(); }
};
int main() {
  A a;
  int* p = a;
  return p != 0;
}
