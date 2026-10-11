template<class Self, class T, class Alloc>
struct Layout {
  using size_type = int;
};

template<class T, class Alloc, template<class, class, class> class LayoutT>
struct Split : LayoutT<Split<T, Alloc, LayoutT>, T, Alloc> {
  using Base = LayoutT<Split<T, Alloc, LayoutT>, T, Alloc>;
  using typename Base::size_type;

  template<class It>
  void construct_at_end_with_size(It first, size_type n);
};

template<class T, class Alloc, template<class, class, class> class LayoutT>
template<class It>
void Split<T, Alloc, LayoutT>::construct_at_end_with_size(It first, size_type n) {
  (void)first;
  (void)n;
}

int main() {
  return 0;
}
