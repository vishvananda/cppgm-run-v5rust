template<class T, T V>
struct integral_constant {
  static const T value = V;
};

template<bool V>
struct bool_constant : integral_constant<bool, V> {};

template<class Alloc>
struct allocator_traits {
  typedef bool_constant<true> propagate_on_container_move_assignment;
};

template<class T>
struct is_nothrow_move_assignable : bool_constant<true> {};

template<class Alloc, class Traits = allocator_traits<Alloc> >
struct move_assign_container
    : integral_constant<bool,
                        Traits::propagate_on_container_move_assignment::value &&
                            is_nothrow_move_assignable<Alloc>::value> {};

namespace N {
template<class T>
struct alloc {};
}

int main()
{
  return move_assign_container<N::alloc<char> >::value ? 0 : 1;
}
