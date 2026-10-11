// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], constructor-template deduction with
// defaulted class-template arguments carried through an alias-template rebind.

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class From, class To>
struct is_convertible {
  static const bool value = true;
};

template<class From, class To, class Ret = void>
struct enable_if_convertible
    : enable_if<is_convertible<From *, To *>::value, Ret> {};

static const unsigned long alignment_value = 0;

template<class T,
         class DifferenceType = long,
         class OffsetType = unsigned long,
         unsigned long Alignment = alignment_value>
struct offset_ptr;

template<class T, class DifferenceType, class OffsetType, unsigned long Alignment>
struct offset_ptr {
  offset_ptr() {}

  template<class U>
  using rebind = offset_ptr<U, DifferenceType, OffsetType, Alignment>;

  template<class U>
  offset_ptr(const offset_ptr<U, DifferenceType, OffsetType, Alignment> &,
             typename enable_if_convertible<U, T>::type * = 0)
  {
  }
};

template<class Ptr, class U, unsigned int Mode>
struct pointer_rebinder;

template<class Ptr, class U>
struct pointer_has_rebind {
  template<class V>
  struct any {
    any(const V &) {}
  };

  template<class X>
  static char test(int, typename X::template rebind<U> *);

  template<class X>
  static int test(any<int>, void *);

  static const bool value = sizeof(test<Ptr>(0, 0)) == 1;
};

template<class Ptr, class U>
struct pointer_rebind_mode {
  static const unsigned int mode =
      (unsigned int)pointer_has_rebind<Ptr, U>::value;
};

template<class Ptr, class U>
struct pointer_rebinder<Ptr, U, 1> {
  typedef typename Ptr::template rebind<U> type;
};

template<class Ptr, class U>
struct pointer_rebind
    : pointer_rebinder<Ptr, U, pointer_rebind_mode<Ptr, U>::mode> {};

template<class VoidPointer>
struct compact_node {
  typedef compact_node<VoidPointer> node;
  typedef typename pointer_rebind<VoidPointer, node>::type node_ptr;
  typedef typename pointer_rebind<VoidPointer, const node>::type const_node_ptr;
  node_ptr parent;
};

template<class VoidPointer>
struct compact_node_traits_impl {
  typedef compact_node<VoidPointer> node;
  typedef typename node::node_ptr node_ptr;
  typedef typename node::const_node_ptr const_node_ptr;
};

template<class VoidPointer, bool Compact>
struct node_traits_dispatch {};

template<class VoidPointer>
struct node_traits_dispatch<VoidPointer, true>
    : compact_node_traits_impl<VoidPointer> {};

template<class VoidPointer, bool OptimizeSize>
struct node_traits : node_traits_dispatch<VoidPointer, OptimizeSize> {};

typedef offset_ptr<void, long, unsigned long, 0> void_ptr;
typedef node_traits<void_ptr, true> traits;
typedef traits::node_ptr node_ptr;
typedef traits::const_node_ptr const_node_ptr;

static bool unique(const_node_ptr)
{
  return true;
}

int main()
{
  node_ptr to_erase;
  return unique(to_erase) ? 0 : 1;
}
