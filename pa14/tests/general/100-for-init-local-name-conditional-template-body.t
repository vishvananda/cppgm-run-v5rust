template<class T> void f(T x) {
  long b = x;
  for (long i = 0; true ? i < b : i > b; ++i) { }
}
int main() { f(0); }
