// Converted-constant narrowing in a specialization's explicit condition is a
// hard diagnostic, even with an ordinary constructor overload available.
struct A {
  template<class T> explicit(sizeof(T)) A(T) {}
  A(int) {}
};
int main() { A a(1); }
