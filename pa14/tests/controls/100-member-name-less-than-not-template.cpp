// N3485 3.4.5p1: in o.hash < lo, `hash` is first looked up in the object's
// class; found as a non-template data member, `<` is less-than even though a
// namespace-scope class template `hash` exists; expect exit 0
template<class T> struct hash {};
struct Operand { unsigned hash; };
static bool f(const Operand& o, unsigned lo, unsigned hi) { return o.hash < lo || o.hash > (hi); }
int main() { Operand o = { 5 }; return f(o, 1, 9) ? 1 : 0; }
