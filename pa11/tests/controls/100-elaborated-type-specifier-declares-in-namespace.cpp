// N3485 3.3.2/7: struct X first named in a parameter-declaration-clause or a
// class member declaration declares X in the enclosing namespace, so both f
// declarations name one function and S::p points to ::X; expect exit 0
void f(struct X* p);
void f(struct X* p) { }
struct S { struct Y* p; int g(); };
struct Y { typedef int t; int v; };
int S::g() { Y::t r = p->v; return r; }
int main() { f(0); Y y; y.v = 0; S s; s.p = &y; return s.g(); }
