template<class T, class U> struct Pair {};
template<class... T> int consume(T...) { return 0; }
struct A { A(int) {} };
struct B { B(int) {} };
template<class... T> struct X : T... {
  template<class... U> X(U... u)
    : T(u)... {
    consume(Pair<T, U>()...);
  }
};
int main() { X<A, B> object(1); }
