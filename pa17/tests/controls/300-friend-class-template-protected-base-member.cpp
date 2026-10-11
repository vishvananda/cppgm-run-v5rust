// N3485 14.5.4p1, 11.2p5, 11.4p1: every Reader<T> is a friend of P, so p.N::x
// (protected in base N, object of type P) is accessible in Reader<P>;
// expect exit 0
class N { protected: int x; public: N() : x(4) {} };
class P : public N { template<class T> friend struct Reader; };
template<class T> struct Reader { static int get(T& p) { return p.N::x; } };
int main() { P p; return Reader<P>::get(p) == 4 ? 0 : 1; }
