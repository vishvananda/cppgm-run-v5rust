// VALIDATION: compile-pass
// A cached function-template result type that contains a concrete invalid
// non-type template argument must still be rejected as a SFINAE candidate.

namespace boost
{
namespace intrusive
{
namespace detail
{
typedef char yes_type;

template<class T>
struct internal_base_hook_bool
{
  template<bool Add>
  struct two_or_three
  {
    yes_type payload[2u + (unsigned)Add];
  };

  template<class U>
  static yes_type test(...);

  template<class U>
  static two_or_three<U::hooktags::is_base_hook> test(int);

  static const unsigned long value = sizeof(test<T>(0));
};

template<class T>
struct internal_base_hook_bool_is_true
{
  static const bool value =
      internal_base_hook_bool<T>::value > sizeof(yes_type) * 2;
};

template<class SupposedValueTraits>
struct is_default_hook_tag
{
  static const bool value = false;
};

template<class SupposedValueTraits, class T,
         bool = is_default_hook_tag<SupposedValueTraits>::value>
struct supposed_value_traits;

template<class SupposedValueTraits, class T>
struct supposed_value_traits<SupposedValueTraits, T, false>
{
  typedef SupposedValueTraits type;
};

template<class SupposedValueTraits, class T,
         bool = internal_base_hook_bool_is_true<SupposedValueTraits>::value>
struct supposed_base_value_traits;

template<class SupposedValueTraits, class T>
struct supposed_base_value_traits<SupposedValueTraits, T, false>
{
  typedef SupposedValueTraits type;
};

template<class T, class SupposedValueTraits>
struct get_value_traits
    : supposed_base_value_traits<
          typename supposed_value_traits<SupposedValueTraits, T>::type,
          T>
{
};
}

struct node
{
};

struct node_traits
{
  typedef node * node_ptr;
  typedef const node * const_node_ptr;
  typedef node node_type;
};

struct hooktags
{
  typedef node_traits node_traits;
  static const int link_mode = 0;
};

template<class... Options>
struct list_member_hook
{
  typedef hooktags hooktags;
  node n;
};

template<class T, class Hook, Hook T::* P>
struct mhtraits
{
  typedef Hook hook_type;
  typedef typename hook_type::hooktags::node_traits node_traits;
  typedef typename node_traits::node_type node;
  typedef typename node_traits::node_ptr node_ptr;
  typedef T & reference;

  static node_ptr to_node_ptr(reference value)
  {
    return &(value.*P).n;
  }
};

template<class Parent, class MemberHook, MemberHook Parent::* PtrToMember>
struct member_hook
{
  typedef mhtraits<Parent, MemberHook, PtrToMember> member_value_traits;
};
}
}

struct value_type
{
  boost::intrusive::list_member_hook<> hook;
};

typedef boost::intrusive::member_hook<
    value_type,
    boost::intrusive::list_member_hook<>,
    &value_type::hook> member_hook_type;

typedef boost::intrusive::detail::get_value_traits<
    value_type,
    member_hook_type::member_value_traits>::type result_type;

int main()
{
  return sizeof(result_type) == 0 ? 1 : 0;
}
