// VALIDATION: compile-pass
// A concrete type produced by an alias-template rebind in a non-dependent
// function parameter must not participate in member-template argument
// deduction.  Only the forwarding-reference argument should deduce U.

template<class PointedType, class DifferenceType, class OffsetType, int OffsetAlignment>
struct Ptr {
  template<class U>
  using rebind = Ptr<U, DifferenceType, OffsetType, OffsetAlignment>;
};

template<class Pointer>
struct Traits {
  template<class U>
  struct rebind_pointer {
    typedef typename Pointer::template rebind<U> type;
  };
};

template<class Pointer, bool IsConst>
struct Iter {
};

template<class T>
struct Vec {
  typedef typename Traits<Ptr<T, long, unsigned long, 0> >::template rebind_pointer<T>::type pointer;
  typedef Iter<pointer, true> const_iterator;

  template<class U>
  int priv_insert(const const_iterator &, U &&) {
    return 1;
  }

  int insert(const_iterator it, const T & value) {
    return priv_insert(it, value);
  }
};

int main() {
  Vec<int> v;
  Vec<int>::const_iterator it;
  int value = 0;
  return v.insert(it, value) == 1 ? 0 : 1;
}
