// N3485 6.8p1, 5.2.2: Base::operator=(Base()); is an expression statement
// calling the base assignment; expect exit 0
struct S { int n; };
struct N : S { void init() { S::operator=(S()); } };
int main() { N x; x.n = 5; x.init(); return x.n == 0 ? 0 : 1; }
