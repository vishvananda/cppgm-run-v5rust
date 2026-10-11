// VALIDATION: compile-pass
// A concrete template-id argument with a cv mismatch must not fall through
// into nested template-id deduction while matching class partials.

template<class First, class Second>
struct pair {
  First first;
  Second second;
};

template<class T>
struct less {};

template<class T, template<class> class Compare>
struct interval {};

template<class Key, class T>
struct impl_map {
  typedef pair<const Key, T> value_type;
};

template<class Derived, class T>
struct base_map {
  typedef interval<T, less> interval_type;
  typedef pair<interval_type, T> interval_mapping_type;
  typedef impl_map<interval_type, T> impl_type;
  typedef typename impl_type::value_type value_type;
};

template<class T>
struct interval_map : base_map<interval_map<T>, T> {
  typedef base_map<interval_map<T>, T> base_type;
  typedef typename base_type::interval_mapping_type interval_mapping_type;
  typedef typename base_type::value_type value_type;
};

template<class Type, class AssociateT>
struct derivative;

template<class Type>
struct derivative<Type, typename Type::interval_mapping_type> {
  static const int value = 1;
};

template<class Type>
struct derivative<Type, typename Type::value_type> {
  static const int value = 2;
};

template<class Type, class AssociateT>
struct derivative {
  static const int value = 0;
};

static_assert(derivative<interval_map<int>, interval_map<int>::interval_mapping_type>::value == 1, "");
static_assert(derivative<interval_map<int>, interval_map<int>::value_type>::value == 2, "");

int main()
{
  return derivative<interval_map<int>, interval_map<int>::value_type>::value == 2 ? 0 : 1;
}
