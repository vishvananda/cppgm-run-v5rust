// expect exit 0
// N3485 14.8.1p9: declaration deduction may extend an explicit pack prefix.
template<class... T> int choose(T...) { return 1; }
template<> int choose<int>(int, long) { return 7; }
template<class... T> int references(T&...) { return 2; }
template<> int references<int>(int&, long&) { return 9; }
template<class... T> int instantiate(T...) { return sizeof...(T); }
template int instantiate<int>(int, long);
int main() {
  int a = 1;
  long b = 2;
  return choose(a, b) == 7 && references(a, b) == 9 &&
         instantiate(a, b) == 2 ? 0 : 1;
}
