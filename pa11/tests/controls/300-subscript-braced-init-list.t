// N3485 13.5.5, 5.2.1p2: x[{...}] calls an overloaded operator[] with the
// braced-init-list; expect exit 0
struct S { int operator[](int i) const { return i * 2; } };
int main() { S s; return s[{3}] == 6 ? 0 : 1; }
