// N3485 5.3.3p1/5.2.3p3/8.2p2: in sizeof(T{}) the operand is an expression
// (braced functional cast), not a type-id; expect exit 0
struct P { int a, b; };
template <class T> int f() { return sizeof(T{}); }
int main() { return (sizeof(double{}) == 8 && sizeof(P{1, 2}) == 8 && f<char>() == 1) ? 0 : 1; }
