// N3485 14p2 (no template declaration at block scope; also 14p4 C linkage,
// 14p5 name conflicts with a function/variable/enumerator, 13.5.3p1 operator=
// template non-member); must be rejected
int main() { template<class T> struct L { }; return 0; }
