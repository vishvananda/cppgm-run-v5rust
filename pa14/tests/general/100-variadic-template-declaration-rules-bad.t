// N3485 14.5.3p5: a pack expansion must name a parameter pack (T... with
// non-pack T); siblings 14.1p11 (class pack not last, pack default), 5.3.3p5,
// 14.5.5p8, 14.7.3p21, 14.3p1; must be rejected
template<class T> void f(T... t);
int main() { return 0; }
