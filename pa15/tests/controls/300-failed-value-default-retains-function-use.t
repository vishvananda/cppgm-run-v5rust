// expect exit 0
// A nonconstant template default discards the candidate, retaining the helper's use.
template<class T> int helper() { return 3; }
template<class T, int N = helper<T>()> int choose(T*) { return N; }
int choose(...) { return 0; }
int main() {
  int* p = 0;
  return choose(p);
}
