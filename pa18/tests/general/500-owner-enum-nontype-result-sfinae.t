// VALIDATION: compile-pass
// N3485 focus: 14.3.2 [temp.arg.nontype], 14.8.2 [temp.deduct],
// 14.8.3 [temp.over]
// Function-template result substitution must validate non-type arguments from
// the concrete owner type, not by reparsing the owner's saved argument text.

namespace owner_enum_sfinae {

typedef char yes_type;

struct default_tag {};
struct member_tag {};

template<class A, class B>
struct is_same
{
  static const bool value = false;
};

template<class A>
struct is_same<A, A>
{
  static const bool value = true;
};

enum algo_types
{
  slist_algorithms = 1
};

enum link_mode_type
{
  normal_link = 0
};

enum base_hook_type
{
  no_base_hook = 0,
  slist_base_hook = 2
};

struct node_traits
{
  struct node {};
};

template<class NodeTraits, class Tag, link_mode_type LinkMode, base_hook_type BaseHookType>
struct hooktags_impl
{
  typedef NodeTraits node_traits;
  typedef Tag tag;
  static const link_mode_type link_mode = LinkMode;
  static const bool is_base_hook = !is_same<Tag, member_tag>::value;
  static const unsigned int type = BaseHookType;
};

template<class HookTags, unsigned int>
struct hook_tags_definer {};

template<class HookTags>
struct hook_tags_definer<HookTags, slist_base_hook>
{
  typedef HookTags default_slist_hook;
};

template<algo_types Algo,
         class NodeTraits,
         class Tag,
         link_mode_type LinkMode,
         base_hook_type BaseHookType>
struct generic_hook
    : hook_tags_definer<
          generic_hook<Algo, NodeTraits, Tag, LinkMode, BaseHookType>,
          is_same<Tag, default_tag>::value ? BaseHookType : no_base_hook>
{
  typedef hooktags_impl<NodeTraits, Tag, LinkMode, BaseHookType> hooktags;
  static const bool is_any_hook = false;
};

template<class T>
struct internal_base_hook_bool
{
  template<bool Add>
  struct two_or_three
  {
    yes_type bytes[2u + (unsigned)Add];
  };

  template<class U>
  static yes_type test(...);

  template<class U>
  static two_or_three<U::hooktags::is_base_hook> test(int);

  static const unsigned value = sizeof(test<T>(0));
};

template<class T>
struct internal_base_hook_bool_is_true
{
  static const bool value = internal_base_hook_bool<T>::value > sizeof(yes_type) * 2;
};

template<class T>
struct internal_any_hook_bool
{
  template<bool Add>
  struct two_or_three
  {
    yes_type bytes[2u + (unsigned)Add];
  };

  template<class U>
  static yes_type test(...);

  template<class U>
  static two_or_three<U::is_any_hook> test(int);

  static const unsigned value = sizeof(test<T>(0));
};

template<class T>
struct internal_any_hook_bool_is_true
{
  static const bool value = internal_any_hook_bool<T>::value > sizeof(yes_type) * 2;
};

}  // namespace owner_enum_sfinae

typedef owner_enum_sfinae::generic_hook<
    (owner_enum_sfinae::algo_types)1,
    owner_enum_sfinae::node_traits,
    owner_enum_sfinae::default_tag,
    (owner_enum_sfinae::link_mode_type)0,
    (owner_enum_sfinae::base_hook_type)2> selected_hook;

static_assert(owner_enum_sfinae::internal_base_hook_bool_is_true<selected_hook>::value,
              "base hook result probe");
static_assert(!owner_enum_sfinae::internal_any_hook_bool_is_true<selected_hook>::value,
              "any hook result probe");

int main()
{
  return owner_enum_sfinae::internal_base_hook_bool_is_true<selected_hook>::value &&
         !owner_enum_sfinae::internal_any_hook_bool_is_true<selected_hook>::value ?
             0 :
             1;
}
