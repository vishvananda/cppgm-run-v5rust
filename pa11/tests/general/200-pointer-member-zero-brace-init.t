// HHC-290
struct S { int* p; S() : p{0} {} };
int main() { S s; return s.p == 0 ? 0 : 1; }
