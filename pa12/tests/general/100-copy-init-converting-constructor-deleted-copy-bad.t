// N3485 8.5, 13.3.1.4: select the conversion, then validate final construction.
struct X {}; struct A { int value; A(X &):value(1){} A(A const &)=delete; A(A &&)=delete; }; int main(){X x; A a=x; return a.value;}
