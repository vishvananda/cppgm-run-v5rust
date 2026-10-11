// N3485 8.5.1p2, 13.3.1.7p1: an aggregate member/array element is
// copy-list-initialized from {1, 2}; selecting an explicit constructor is
// ill-formed. Must reject.
struct N { int v; explicit N(int a, int b) : v(a + b) {} };
struct A { N n; int k; };
int main() { A a = {{3, 4}, 5}; return a.n.v; }
