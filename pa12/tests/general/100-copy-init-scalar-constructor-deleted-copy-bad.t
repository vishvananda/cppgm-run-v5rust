// N3485 8.5, 13.3.1.4: select the conversion, then validate final construction.
struct A { int value; A(int n):value(n){} A(A const &)=delete; A(A &&)=delete; }; int main(){A a=7;return a.value;}
