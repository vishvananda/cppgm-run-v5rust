// A specialization's bases are its template's with the arguments substituted
// (14.7.1, 10.2): a member of `wrap<own>`, `wrap<T> : T`, is own's, through a
// template-id, a typedef or alias of one, nested specializations and a pack's
// expansion; so `bool(wrap<...>::value)` is a functional cast (14.3p2 takes a
// type-id only where one can be) and `W::type * p` declares p. expect exit 0
struct own { typedef int type; static const int value = 7; struct In { static const int w = 2; }; };
struct other { typedef long ltype; };
template<class T> struct wrap : T {};
template<class... Ts> struct all : Ts... {};
template<bool B> struct sel { static const int value = B ? 7 : 0; };
typedef wrap<wrap<own> > WW;
using AW = wrap<own>;
typedef AW AW2;
int x = 3;
WW::type* gp = &x;
int f() { if (AW::type* q = &x) return *q; return 0; }
int main() {
  wrap<wrap<own> >::type* p = &x;
  AW2::type* r = &x;
  all<other, own>::ltype* lp = 0;
  all<other, own>::type* tp = &x;
  int s = sel<bool(wrap<wrap<own> >::value)>::value;
  int w = wrap<own>::In::w;
  int u = sel<bool(wrap<AW2>::value)>::value;
  return *p + *r + *gp + f() + (lp == 0) + *tp + s + w + u - 3 * 5 - 1 - 7 - 2 - 7;
}
