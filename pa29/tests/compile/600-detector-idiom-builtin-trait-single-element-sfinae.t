// Reduced from libstdc++ std::tuple/std::pair construction constraints
// (forward_as_tuple / piecewise_construct), which wrap is_constructible /
// is_assignable in the __and_/__or_ detector idiom. A single-element pack whose
// builtin-trait element resolves to false must SFINAE the primary overload away
// (enable_if_t<false> has no ::type) instead of tripping a hard type-argument
// text-fallback audit.
template<class T, T v>
struct integral_constant {
  static constexpr T value = v;
  constexpr operator T() const { return value; }
};
using true_type = integral_constant<bool, true>;
using false_type = integral_constant<bool, false>;
template<bool b> using bool_constant = integral_constant<bool, b>;

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

template<class T, class... A>
struct is_constructible : bool_constant<__is_constructible(T, A...)> {};

struct C {
  C();
  C(const C&);
  C(C&&);
};

// rvalue reference cannot bind an lvalue: is_constructible<C&&, C&> is false,
// so this single-element conjunction must reduce to false_type.
static_assert(!conjunction<is_constructible<C&&, C&> >::value,
              "C&& is not constructible from C&");
// copy construction from an lvalue is available: single-element conjunction
// reduces to true_type.
static_assert(conjunction<is_constructible<C, C&> >::value,
              "C is constructible from C&");

int main()
{
  return 0;
}
