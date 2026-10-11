// N3485 5.16p4-p6, 5.2.5p4: c ? mkh(1).a : mkh(2).b (class-type members of
// class prvalues) is a well-formed conditional of type A; expect exit 0
struct A { int val; A(int v) : val(v) {} ~A() {} };
struct H { A a; A b; H(int v) : a(v), b(v + 1) {} };
H mkh(int v) { return H(v); }
volatile int c = 0;
int main() { A x = c ? mkh(1).a : mkh(2).b; return x.val == 3 ? 0 : 1; }
