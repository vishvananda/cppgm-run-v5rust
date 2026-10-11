// VALIDATION: compile-pass
// N3485 focus: 14.5.5.1 [temp.class.spec.match], 14.8.2 [temp.deduct].
// A cached partial-specialization pattern containing a dependent qualified
// member must not retain resolution state from an earlier match scope.

namespace library {

struct false_type {
  static constexpr bool value = false;
};

struct true_type {
  static constexpr bool value = true;
};

template<class A, class B>
struct is_same : false_type {
};

template<class A>
struct is_same<A, A> : true_type {
};

template<bool Condition, class T = void>
struct enable_if_c {
  using type = T;
};

template<class T>
struct enable_if_c<false, T> {
};

template<class Condition, class T = void>
struct enable_if : enable_if_c<Condition::value, T> {
};

namespace mpl {

template<bool Value>
struct bool_ {
  static constexpr bool value = Value;
};

template<class T>
struct not_ : bool_<!T::value> {
};

template<bool Value, class T>
struct and_impl : false_type {
};

template<class T>
struct and_impl<true, T> : bool_<T::value> {
};

template<class A, class B>
struct and_ : and_impl<A::value, B> {
};

}

namespace bimaps {
namespace relation {

namespace member_at {
struct left {
};
}

namespace support {

template<class Tag, class Relation, class Enable = void>
struct member_with_tag {
};

template<class Relation>
struct member_with_tag<member_at::left, Relation, void> {
  using type = member_at::left;
};

template<class Tag, class Relation>
struct member_with_tag<
    Tag,
    Relation,
    typename enable_if<
        mpl::and_<
            mpl::not_<is_same<Tag, member_at::left>>,
            is_same<Tag, typename Relation::left_tag>>>::type> {
  using type = member_at::left;
};

template<class Tag, class Relation, class Enable = void>
struct is_tag_of_member_at_left : mpl::bool_<false> {
};

template<class Tag, class Relation>
struct is_tag_of_member_at_left<
    Tag,
    Relation,
    typename enable_if<
        is_same<typename member_with_tag<Tag, Relation>::type,
                member_at::left>>::type> : mpl::bool_<true> {
};

}
}

namespace detail {

template<class Tag, class BimapCore, class Enable = void>
struct core_iterator_type_by {
};

template<class Tag, class BimapCore>
struct core_iterator_type_by<
    Tag,
    BimapCore,
    typename enable_if<
        relation::support::is_tag_of_member_at_left<Tag, BimapCore>>::type> {
  using type = int;
};

template<class Tag, class BimapCore>
struct map_view_iterator_adaptor {
  using type = typename core_iterator_type_by<Tag, BimapCore>::type;
};

template<class Core>
struct left_map_view_extra_typedefs {
  using left_iterator = typename map_view_iterator_adaptor<
      relation::member_at::left,
      Core>::type;
};

}

template<class T>
struct bimap_core {
  using left_tag = relation::member_at::left;
};

template<class T>
struct bimap : detail::left_map_view_extra_typedefs<bimap_core<T>> {
  using left_tag = relation::member_at::left;
};

namespace support {

template<class Tag, class SymmetricType, class Enable = void>
struct data_type_by {
};

template<class Tag, class SymmetricType>
struct data_type_by<
    Tag,
    SymmetricType,
    typename enable_if<
        relation::support::is_tag_of_member_at_left<Tag, SymmetricType>>::type> {
  using type = typename SymmetricType::left_tag;
};

}
}
}

int main() {
  library::bimaps::support::data_type_by<
      library::bimaps::relation::member_at::left,
      library::bimaps::bimap<int>>::type value;
  (void)value;
  return 0;
}
