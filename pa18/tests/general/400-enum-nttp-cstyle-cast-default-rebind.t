namespace boost {
namespace interprocess {

template<class T, class Difference, class Size, unsigned long Alignment>
struct offset_ptr
{};

}

namespace intrusive {

enum algo_types {
  RbTreeAlgorithms = 5
};

enum link_mode_type {
  normal_link = 0
};

enum base_hook_type {
  NoBaseHookId = 0,
  RbTreeBaseHookId = 3
};

struct dft_tag {};
struct member_tag {};
typedef char yes_type;

template<class VoidPointer, bool OptimizeSize>
struct rbtree_node_traits
{
  struct node {};
  typedef node * node_ptr;
};

template <class A, class B>
struct is_same
{
  static const bool value = false;
};

template <class A>
struct is_same<A, A>
{
  static const bool value = true;
};

template<bool C, class T, class F>
struct if_c
{
  typedef T type;
};

template<class T, class F>
struct if_c<false, T, F>
{
  typedef F type;
};

template<class Prev, class Next>
struct do_pack
{
  typedef typename Next::template pack<Prev> type;
};

template<class Prev>
struct do_pack<Prev, void>
{
  typedef Prev type;
};

template<class DefaultOptions, class O1 = void, class O2 = void>
struct pack_options
{
  typedef typename do_pack
      < typename do_pack<DefaultOptions, O1>::type
      , O2
      >::type type;
};

struct rbtree_defaults
{
  typedef void proto_value_traits;
  typedef int key_of_value;
  typedef int compare;
  typedef unsigned long size_type;
  static const bool constant_time_size = true;
  typedef int header_holder_type;
};

template<class BaseHook>
struct base_hook
{
  template<class Base>
  struct pack : Base
  {
    typedef BaseHook proto_value_traits;
  };
};

template <class HookTags, unsigned int>
struct hook_tags_definer {};

template <class HookTags>
struct hook_tags_definer<HookTags, RbTreeBaseHookId>
{
  typedef HookTags default_rbtree_hook;
};

template<class Node, class Tag, base_hook_type BaseHookType>
struct node_holder : Node
{};

template<class NodeTraits, class Tag, link_mode_type LinkMode, base_hook_type BaseHookType>
struct hooktags_impl
{
  typedef NodeTraits node_traits;
  typedef Tag tag;
  static const link_mode_type link_mode = LinkMode;
  static const bool is_base_hook = !is_same<Tag, member_tag>::value;
  static const unsigned int type = BaseHookType;
};

template <algo_types Algo, class NodeTraits, class Tag, link_mode_type LinkMode, base_hook_type BaseHookType>
class generic_hook
{
public:
  typedef hooktags_impl<NodeTraits, Tag, LinkMode, BaseHookType> hooktags;
};

namespace detail {

template <class T>
struct internal_base_hook_bool
{
  template<bool Add>
  struct two_or_three { yes_type _[2u + (unsigned)Add]; };
  template <class U> static yes_type test(...);
  template <class U> static two_or_three<U::hooktags::is_base_hook> test(int);
  static const unsigned long value = sizeof(test<T>(0));
};

template <class T>
struct internal_base_hook_bool_is_true
{
  static const bool value =
      internal_base_hook_bool<T>::value > sizeof(yes_type) * 2;
};

template <class T>
struct internal_any_hook_bool
{
  template<bool Add>
  struct two_or_three { yes_type _[2u + (unsigned)Add]; };
  template <class U> static yes_type test(...);
  template <class U> static two_or_three<U::is_any_hook> test(int);
  static const unsigned long value = sizeof(test<T>(0));
};

template <class T>
struct internal_any_hook_bool_is_true
{
  static const bool value =
      internal_any_hook_bool<T>::value > sizeof(yes_type) * 2;
};

template<class BaseHook>
struct concrete_hook_base_value_traits
{
  typedef typename BaseHook::hooktags tags;
  typedef typename tags::node_traits type;
};

template<class T, class BaseHook, bool = internal_any_hook_bool_is_true<BaseHook>::value>
struct get_base_value_traits;

template<class T, class BaseHook>
struct get_base_value_traits<T, BaseHook, false>
  : concrete_hook_base_value_traits<BaseHook>
{};

template<class T, class BaseHook>
struct get_base_value_traits<T, BaseHook, true>
{
  typedef int type;
};

template<class SupposedValueTraits, class T, bool = internal_base_hook_bool_is_true<SupposedValueTraits>::value>
struct supposed_base_value_traits;

template<class BaseHook, class T>
struct supposed_base_value_traits<BaseHook, T, true>
  : get_base_value_traits<T, BaseHook>
{};

template<class SupposedValueTraits, class T>
struct supposed_base_value_traits<SupposedValueTraits, T, false>
{
  typedef int type;
};

template<class T, class SupposedValueTraits>
struct get_value_traits
  : supposed_base_value_traits<SupposedValueTraits, T>
{};

}

template<class ValueTraits, class Key, class Compare, class Size, bool ConstantTimeSize, class HeaderHolder>
struct multiset_impl
{
  typedef int iterator;
};

template<class T, class O1 = void, class O2 = void>
struct make_multiset
{
  typedef typename pack_options<rbtree_defaults, O1, O2>::type packed_options;
  typedef typename detail::get_value_traits
      < T
      , typename packed_options::proto_value_traits
      >::type value_traits;
  typedef multiset_impl
      < value_traits
      , typename packed_options::key_of_value
      , typename packed_options::compare
      , typename packed_options::size_type
      , packed_options::constant_time_size
      , typename packed_options::header_holder_type
      > implementation_defined;
  typedef implementation_defined type;
};

}
}

struct block_ctrl;

typedef boost::intrusive::generic_hook
  < (boost::intrusive::algo_types)5
  , boost::intrusive::rbtree_node_traits
      < boost::interprocess::offset_ptr<void, long, unsigned long, 0>
      , true
      >
  , boost::intrusive::dft_tag
  , (boost::intrusive::link_mode_type)0
  , (boost::intrusive::base_hook_type)3
  > hook_type;

struct block_ctrl : hook_type {};

typedef boost::intrusive::make_multiset
  < block_ctrl
  , boost::intrusive::base_hook<hook_type>
  >::type set_type;

int main()
{
  return sizeof(set_type) > 0 ? 0 : 1;
}
