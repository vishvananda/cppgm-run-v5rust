// N3485 3.9p10/12.1p6: a union whose implicit default constructor initializes
// one member by its brace-or-equal-initializer is a literal type even with an
// (uninitialized) anonymous union member; expect exit 0
union K0 { long long m4 = 1; union { char m1; }; };
constexpr K0 k0{};
int main() { return k0.m4 == 1 ? 0 : 1; }
