template<class V, class D>
struct block_size {
  static const D value = sizeof(V) < 256 ? 4096 / sizeof(V) : 16;
};

template<class T>
struct alloc_traits {
  typedef long difference_type;
};

// A defaulted non-type argument expression must keep structured syntax after
// substituting preceding type template arguments.
template<class V, class P, class D, D BS = block_size<V, D>::value>
struct iter {
  P p;
};

template<class T, class Alloc>
struct holder {
  typedef T value_type;
  typedef typename alloc_traits<Alloc>::difference_type difference_type;
  typedef iter<value_type, value_type *, difference_type> iterator;
  iterator move_and_check(iterator f, iterator l, iterator r);
};

template<class T, class Alloc>
typename holder<T, Alloc>::iterator
holder<T, Alloc>::move_and_check(iterator f, iterator l, iterator r) {
  return f;
}

int main() {
  holder<int, int> h;
  h.move_and_check(holder<int, int>::iterator(),
                   holder<int, int>::iterator(),
                   holder<int, int>::iterator());
  return 0;
}
