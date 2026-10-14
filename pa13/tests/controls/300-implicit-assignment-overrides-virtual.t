// N3485 10.3p2, 12.8p18 (footnote): B's implicitly-declared copy assignment
// B::operator=(const B&) overrides a virtual A::operator=(const B&), so a call
// through A& dispatches to it (which assigns the A base by A::operator=(const
// A&), so calls stays 0); expect exit 0
int calls;
struct B;
struct A { virtual A& operator=(const B&) { ++calls; return *this; } virtual ~A() {} };
struct B : A { int v; };
int main() { B b1, b2; b2.v = 7; b1.v = 0; A& a = b1; a = b2; return (calls == 0 && b1.v == 7) ? 0 : 1; }
