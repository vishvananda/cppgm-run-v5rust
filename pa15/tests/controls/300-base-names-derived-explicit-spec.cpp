// N3485 14.7.1p1, 14.7.3p1: naming F<void>& in a member declaration of base
// BF<void> does not instantiate F<void>; explicit specialization F<void> may
// derive from BF<void> (libstdc++ <future>); expect exit 0
template<class R> struct F;
template<class R> struct BF { BF() {} void g(F<R>&); int x; };
template<class R> struct F : BF<R> { };
template<> struct F<void> : BF<void> { };
int main() { F<void> f; f.x = 1; return f.x == 1 ? 0 : 1; }
