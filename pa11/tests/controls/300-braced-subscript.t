// N3485 5.2.1p1/13.5.5p1: x[{a, b}] calls x.operator[]({a, b}) with a
// braced-init-list argument; expect exit 0
struct P { int a, b; };
struct Q { int operator[](P p) const { return p.a * 10 + p.b; } };
int main() { Q q; return q[{4, 5}] == 45 ? 0 : 1; }
