// N3485 7.1.5p4, 9.5p5: an enclosing constexpr constructor must initialize
// one member of an anonymous union; expect reject
struct S { union { int x; float y; }; constexpr S() {} };
int main() { return 0; }
