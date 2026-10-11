// VALIDATION: compile-pass
// N3485 focus: 14.3.2 [temp.arg.nontype], 14.5.7 [temp.alias]

template<class T, T V>
struct integral_constant {
  static constexpr T value = V;
  typedef T value_type;
  typedef integral_constant type;
};

template<bool B>
using bool_constant = integral_constant<bool, B>;

typedef integral_constant<bool, true> true_type;

template<class T>
struct builtin_trait : bool_constant<__is_constructible(T, T)> {
};

template<class T>
struct allocator {
  typedef T value_type;
};

template<class Alloc>
struct move_insertable : builtin_trait<typename Alloc::value_type> {
};

static_assert(__is_base_of(true_type, move_insertable<allocator<int> >),
              "bool alias base should preserve the bool non-type argument type");

int main()
{
  return 0;
}
