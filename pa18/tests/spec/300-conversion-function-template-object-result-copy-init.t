// VALIDATION: compile-pass
// N3485 focus: 13.3.1.4 [over.match.copy], 14.8.2.3 [temp.deduct.conv]
struct A { float f; };
A seed = {0};
struct X { A a; template<class T> operator T() const { return a; } } const x = {};
int main() { A value = x; (void)value; }
