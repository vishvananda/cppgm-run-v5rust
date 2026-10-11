// N3485 5.1.1p8/7.1.6.2: decltype(expr)::f() names the static member function
// f of the decltype type; expect exit 0
struct S { static int f() { return 7; } };
S make();
int main() { return decltype(make())::f() == 7 ? 0 : 1; }
