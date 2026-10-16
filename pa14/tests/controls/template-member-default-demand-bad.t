// expect reject
template<class T> struct A {
  A(int value = T::missing) {}
};
int main() { A<int> a; return 0; }
