template<class...> struct _And { static constexpr bool value = true; };
struct less_tag {};
template<class...> struct desugars_to {};
template<class...> struct has_default_three_way_comparator {
  static constexpr bool value = true;
};
template<bool, class T = void> struct enable_if {};
template<class T> struct enable_if<true, T> { using type = T; };
template<class... Ts> using enable_if_t = typename enable_if<true, int>::type;

template<class Comparator, class LHS, class RHS, class = void>
struct comparator {};

template<class Comparator, class LHS, class RHS>
struct comparator<
    Comparator,
    LHS,
    RHS,
    enable_if_t<_And<
        desugars_to<less_tag, Comparator, LHS, RHS>,
        has_default_three_way_comparator<LHS, RHS> >::value> > {
  __attribute__((visibility("hidden"))) comparator(const Comparator&) {}
};
