// HHC-138
template<class T> struct traits { struct cat {}; };
struct IterOps { template<class I> static int adv(I&, int, const I&, typename traits<I>::cat); };
template<class I>
int f(I& it, int n, const I& end) {
  return IterOps::adv(it, n, end, typename traits<I>::cat());
}
