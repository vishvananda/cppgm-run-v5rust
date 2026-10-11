// N3485 8.5, 13.3.1.4: select the conversion, then validate final construction.
struct X; struct A { int value; A(int n):value(n){} A(X const &); }; struct X { operator A(){return A(2);} operator A const(){return A(3);} }; A::A(X const &):value(1){} int main(){X x; A a=x; return a.value;}
