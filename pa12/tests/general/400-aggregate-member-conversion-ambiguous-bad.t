// N3485 8.5, 13.3.1.4: select the conversion, then validate final construction.
struct X; struct A { int value; A(int n): value(n) {}  A(X &); }; struct X {  operator A()  { return A(2); } }; A::A(X &): value(1) {} struct Holder { A member; }; int main() { X x; Holder h = {x}; A &a = h.member; return a.value; }
