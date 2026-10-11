// N3485 8.5, 13.3.1.4: select the conversion, then validate final construction.
struct A { int value; A(int n):value(n){} template<class T> A(T &):value(1){} }; struct X { template<class T> operator T() { return T(2); } }; int main(){X x; A a=x; return a.value;}
