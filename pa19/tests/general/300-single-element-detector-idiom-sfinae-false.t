template<class T, T v>
struct integral_constant {
  static constexpr T value = v;
  constexpr operator T() const { return value; }
};
using true_type = integral_constant<bool, true>;
using false_type = integral_constant<bool, false>;

template<bool> struct enable_if {};
template<> struct enable_if<true> { using type = void; };
template<bool B> using enable_if_t = typename enable_if<B>::type;

template<class T, class...> using first_t = T;

template<class... Bn>
auto and_fn(int) -> first_t<true_type, enable_if_t<bool(Bn::value)>...>;
template<class... Bn>
auto and_fn(...) -> false_type;
template<class... Bn>
struct conjunction : decltype(and_fn<Bn...>(0)) {};

template<class... Bn>
auto or_fn(int) -> first_t<false_type, enable_if_t<!bool(Bn::value)>...>;
template<class... Bn>
auto or_fn(...) -> true_type;
template<class... Bn>
struct disjunction : decltype(or_fn<Bn...>(0)) {};

struct yes_tag {};
template<class T> struct is_yes : false_type {};
template<> struct is_yes<yes_tag> : true_type {};

int main()
{
  // A single-element detector idiom whose only element resolves to false: the
  // primary overload's enable_if_t<false> must SFINAE away rather than trip a
  // hard text-fallback audit. (Reduced from libstdc++ __and_/__or_ used by
  // std::tuple/std::pair construction constraints, where the single-element
  // pack lost its structured syntax and reached the explicit-type-argument
  // audit instead of a clean substitution failure.)
  bool a = conjunction<is_yes<int> >::value;     // false (1 element)
  bool b = conjunction<is_yes<yes_tag> >::value; // true  (1 element)
  bool c = conjunction<false_type>::value;       // false
  bool d = conjunction<true_type>::value;        // true
  bool e = disjunction<is_yes<int> >::value;     // false
  bool f = disjunction<is_yes<yes_tag> >::value; // true
  return (!a && b && !c && d && !e && f) ? 0 : 1;
}
