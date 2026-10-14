// Itanium ABI 2.4: a derived member is placed at the base's dsize, reusing the
// tail padding of a non-POD base; struct B : A{int b} with A{vptr,int} has
// sizeof 16 and b at offset 12; expect exit 0
struct A { virtual ~A() {} int a; };
struct B : A { int b; };
int main() { B x; return sizeof(B) == 16 && (char*)&x.b - (char*)&x == 12 ? 0 : 1; }
