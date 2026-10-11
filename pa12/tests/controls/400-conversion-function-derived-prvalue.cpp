// N3485 8.5p16 (last sub-bullet), 13.3.1.4: A copy-initialized from P whose
// operator D() returns a D prvalue: the result is then copied by A(const A&);
// expect exit 0
struct A { int val; A() : val(1) {} A(const A& o) : val(o.val + 10) {} };
struct D : A { };
struct P { operator D() { D d; d.val = 7; return d; } };
A backp(P& p) { return p; }
int main() { P p; return backp(p).val == 17 ? 0 : 1; }
