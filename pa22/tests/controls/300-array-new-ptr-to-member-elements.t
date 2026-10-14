// N3485 5.3.4p1/p5: array new of pointer-to-member elements; expect exit 0
struct A { int x; };
int main() { int n = 3; int A::** pp = new int A::*[n]; pp[2] = &A::x; A a; a.x = 4; return a.*pp[2] == 4 ? 0 : 1; }
