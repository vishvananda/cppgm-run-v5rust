// expect reject
// Instantiating the class checks the fixed call inside its member declaration,
// even though the comma's other operand still names the member's own parameter.
void take(int);
struct X {};
template<class T> struct A {
  template<class U> auto f() -> decltype(take(T()), void(U()));
};
int main() { A<X> a; return 0; }
