// N3485 8.5.4p3, p7: double -> int in an aggregate's braced initializer is a
// narrowing conversion: ill-formed; expect reject
struct S { int i; };
int main() { double d = 1; S s{d}; return s.i - 1; }
