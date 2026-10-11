template<class V, class D>
struct Block {
  static const D value = 4;
};

template<class V, class P, class R, class M, class D, D B = Block<V, D>::value>
struct Iter {
  Iter() {}
  static const D block_size;
};

template<class V, class P, class R, class M, class D, D B>
const D Iter<V, P, R, M, D, B>::block_size = Block<V, D>::value;

int main() {
  return 0;
}
