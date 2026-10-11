// N3485 focus: [temp.dep.type] and [temp.arg.type].
// Source witness lookup must preserve a dependent qualified member type whose
// owner is a local typedef inside the current class template.
template<class T>
struct traits {
  typedef T value_type;

  template<class U>
  struct rebind {
    typedef traits<U> other;
  };
};

template<class Traits, class T>
using rebind_alloc = typename Traits::template rebind<T>::other;

template<class A, class B>
struct is_same {
  static const bool value = false;
};

template<class A>
struct is_same<A, A> {
  static const bool value = true;
};

template<class Alloc>
struct check_valid_allocator {
  typedef traits<Alloc> Traits;
  static_assert(is_same<traits<Alloc>,
                        rebind_alloc<Traits,
                                     typename Traits::value_type> >::value,
                "same type");
};

check_valid_allocator<int> value;

int main()
{
  (void)value;
  return 0;
}
