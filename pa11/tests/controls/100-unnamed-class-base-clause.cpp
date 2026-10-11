// N3485 9p1: class-head-name is optional, so an unnamed class may have a
// base-clause (struct : B { } x;, as in libstdc++ debug/formatter.h);
// expect exit 0
struct B { int b; };
struct : B { int m; } v;
int main() { v.b = 1; v.m = 2; return v.b + v.m == 3 ? 0 : 1; }
