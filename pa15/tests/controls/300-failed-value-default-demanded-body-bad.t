// expect reject
// The default fails constant evaluation, but the helper body still has to be valid.
template<class T> int helper() { return T::bad; }
template<class T, int N = helper<T>()> int choose(T*) { return N; }
int choose(...) { return 0; }
int main() {
  int* p = 0;
  return choose(p);
}
