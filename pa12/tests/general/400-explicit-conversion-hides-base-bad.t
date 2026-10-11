// N3485 8.5, 13.3.1.4: select the conversion, then validate final construction.
struct A{int value;A(int n):value(n){}};struct B{operator A(){return A(2);}};struct X:B{explicit operator A(){return A(3);}};int main(){X x;A a=x;return a.value;}
