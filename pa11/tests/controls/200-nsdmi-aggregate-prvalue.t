// N3485 9.2p4/8.5.4p3: an NSDMI S2 m = S2{E(1), E(2)}; aggregate-initializes
// the S2 prvalue (no constructor call); expect exit 0
struct E { int v; E(int x) : v(x) {} };
struct S2 { E a; E b; };
struct H { S2 m = S2{E(1), E(2)}; };
int main() { H h; return h.m.b.v == 2 ? 0 : 1; }
