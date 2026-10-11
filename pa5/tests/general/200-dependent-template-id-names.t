// N3485 14.2p4, 14.6p2: box<int, X::num < Y::num>, typename X::type and
// X::template get<int>() parse as written in a template (full program: exit
// 14)
template<class T, bool B> struct box { static const int b = B; };
struct R1 { static const int num = 1; typedef int type; template<class U> static int get() { return 10; } };
struct R2 { static const int num = 2; };
template<class X, class Y> struct cmp : box<int, X::num < Y::num> {
  typename X::type value;
  int f() { return X::template get<int>() + (Y::num) + 1; }
};
int main() { cmp<R1, R2> c; return c.f() + cmp<R1, R2>::b; } // 14
