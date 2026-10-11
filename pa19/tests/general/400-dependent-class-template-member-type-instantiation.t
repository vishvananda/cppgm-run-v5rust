template<class Ptr, class T>
struct rebind_pointer {
  typedef T * type;
};

template<class Ptr, class T>
using rebind_pointer_t = typename rebind_pointer<Ptr, T>::type;

template<class T, class Ptr>
struct node {};

template<class P>
struct base {};

template<class Alloc>
struct allocator_traits {
  typedef void * void_pointer;
};

template<class T, class Alloc>
struct table {
  typedef allocator_traits<Alloc> alloc_traits;
  typedef typename alloc_traits::void_pointer void_pointer;
  typedef node<T, void_pointer> node_type;
  typedef rebind_pointer_t<void_pointer, node_type> node_pointer;
  typedef base<node_pointer> first_node;
};

template<class T, class Alloc>
struct holder {
  typedef typename table<T, Alloc>::first_node type;
};

int main() {
  holder<int, int>::type value;
  (void)value;
  return 0;
}
