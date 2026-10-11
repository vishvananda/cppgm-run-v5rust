namespace std {
template<class T>
struct remove_reference {
  typedef T type;
};

template<class T>
struct remove_reference<T &> {
  typedef T type;
};

template<class T>
struct remove_reference<T &&> {
  typedef T type;
};

template<class T>
T &&forward(typename remove_reference<T>::type &value) {
  return static_cast<T &&>(value);
}

template<class Alloc>
struct allocator_traits {
  template<class U>
  using rebind_alloc = typename Alloc::template rebind<U>::other;

  template<class U>
  struct rebind_traits {
    typedef typename rebind_alloc<U>::pointer pointer;
  };
};
}

template<class T>
struct A {
  typedef T value_type;
  typedef T *pointer;

  template<class U>
  struct rebind {
    typedef A<U> other;
  };

  explicit A(int) {}

  template<class U>
  A(const A<U> &) {}
};

template<class Allocator, unsigned long Alignment>
struct adaptor : Allocator {
  typedef std::allocator_traits<Allocator> traits;
  typedef typename traits::template rebind_alloc<char> char_alloc;
  typedef typename traits::template rebind_traits<char> char_traits;
  typedef typename char_traits::pointer char_ptr;
  typedef typename Allocator::value_type value_type;

  adaptor() : Allocator(0) {}

  template<class A>
  explicit adaptor(A &&alloc) : Allocator(std::forward<A>(alloc)) {}
};

int main() {
  adaptor<A<int>, 1> value(5);
  (void)value;
  return 0;
}
