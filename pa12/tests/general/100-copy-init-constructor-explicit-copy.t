// N3485 8.5, 13.3.1.4: select the conversion, then validate final construction.
struct X {}; struct A { int value; A(X &):value(1){} explicit A(A const &a):value(a.value){} }; int main(){X x; A a=x; return a.value == 1 ? 0 : 1;}
