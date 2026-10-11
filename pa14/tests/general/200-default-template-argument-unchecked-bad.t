// N3485 14.1p9-p10, 3.3.9p1: a default template-argument is checked where it
// appears: Nope is undeclared (also: naming a later parameter, or the template
// itself); must be rejected
template<class T = Nope> struct S { };
int main() { return 0; }
