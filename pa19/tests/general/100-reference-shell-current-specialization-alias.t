template<class A>
struct traits {
  typedef typename A::pointer pointer;
  typedef typename A::const_pointer const_pointer;
};

template<class C, bool IsConst>
struct Iter;

template<class C>
struct Iter<C, false> {
  typedef typename C::storage_pointer ptr;
  ptr p;
};

template<class C>
struct Iter<C, true> {
  typedef typename C::const_storage_pointer ptr;
  ptr p;
};

template<class T>
struct Alloc {
  typedef T* pointer;
  typedef const T* const_pointer;
};

template<class A>
struct Vec {
  typedef traits<A> storage_traits;
  typedef typename storage_traits::pointer storage_pointer;
  typedef typename storage_traits::const_pointer const_storage_pointer;
  typedef Iter<Vec, false> pointer;
  typedef Iter<Vec, true> const_pointer;
  pointer begin;
};

int main() {
  Vec<Alloc<int> > v;
  (void)v;
  return 0;
}
