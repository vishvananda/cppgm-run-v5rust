// N3485 12.9p3: from Q(P p = P(7)) only the signature Q(P) is inherited (a
// parameterless form is not), so IQ a; uses IQ's implicit default constructor
// unambiguously; expect exit 0
struct P { int id; explicit P(int i) : id(i) {} };
struct Q { int q; Q(P p = P(7)) : q(p.id) {} };
struct IQ : Q { using Q::Q; };
int main() { IQ a; return a.q == 7 ? 0 : 1; }
