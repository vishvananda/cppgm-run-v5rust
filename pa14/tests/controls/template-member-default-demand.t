// expect exit 0
template<class T> struct A {
  int n;
  A(int value = sizeof(T), int unused = T::missing) : n(value) {}
  int get(int value = T::missing, int extra = sizeof(T)) { return value + extra; }
};
int main() {
  A<int> a(3, 0);
  A<char> b(1, 0);
  return a.get(2) + a.get(3) + b.get(2) - 16;
}
