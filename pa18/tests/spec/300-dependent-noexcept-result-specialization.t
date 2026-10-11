// VALIDATION: compile-pass
// A concrete detector result must not be replaced by stale dependent source
// metadata while recovering an instantiated function result type.

template<bool Value> struct constant { static constexpr bool value = Value; };

template<class T> T&& declval() noexcept;
template<class T> void operation(T&) noexcept;

template<class T, bool Value = noexcept(operation(declval<T&>()))>
constant<Value> probe(int);
template<class T> constant<false> probe(...);

template<class T>
struct detector
{
  using type = decltype(probe<T>(0));
};

static_assert(detector<int>::type::value,
              "the concrete detector result must remain true");

int main() { return 0; }
