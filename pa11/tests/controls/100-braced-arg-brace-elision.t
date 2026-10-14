// N3485 13.3.3.1.5p4 + 8.5.1p11: a braced-init-list argument converts to an
// aggregate parameter it can aggregate-initialize, brace elision included;
// expect exit 0
struct S { int a[2]; };
int take(const S& s) { return s.a[0] + s.a[1]; }
int main() { return take({2, 3}) == 5 ? 0 : 1; }
