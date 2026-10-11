// VALIDATION: compile-pass
// A synthesized qualified declval call remains explicitly unevaluated when
// its detector result is used as a base class.

namespace local
{
template<class T> T&& declval() noexcept;
template<bool Value> struct constant { static constexpr bool value = Value; };
}

template<class T> void operation(T&) noexcept;

template<class T, bool Value = noexcept(operation(local::declval<T&>()))>
local::constant<Value> probe(int);

template<class T>
struct detector
{
  using type = decltype(probe<T>(0));
};

template<class T> struct trait : detector<T>::type {};

static_assert(trait<int>::value,
              "the inherited detector result must remain true");

int main() { return 0; }
