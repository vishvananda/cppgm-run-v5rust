struct S { int x; ~S() {} };
int f(S value) { return __builtin_bit_cast(int, value); }
