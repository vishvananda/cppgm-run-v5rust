// N3485 15.2p2, 12.4: when one element destructor of a std::initializer_list
// backing array throws, the remaining elements are still destroyed (3 2 1 then
// handler 5), as for a named array; expect exit 0
namespace std {
template<class E> class initializer_list {
  const E* b; decltype(sizeof 0) n;
  constexpr initializer_list(const E* p, decltype(sizeof 0) s) : b(p), n(s) {}
public:
  constexpr initializer_list() : b(0), n(0) {}
  decltype(sizeof 0) size() const { return n; }
};
}
int h, cd;
struct X { int x; X(int v) : x(v) {} X(const X& o) : x(o.x) {} ~X() noexcept(false) { h = h * 10 + x; if (--cd == 0) throw 5; } };
int f(std::initializer_list<X> l) { return (int)l.size(); }
int main() { cd = 1; try { f({ X(1), X(2), X(3) }); } catch (int e) { h = h * 10 + e; } return h == 3215 ? 0 : 1; }
