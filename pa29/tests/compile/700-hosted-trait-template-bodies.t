namespace std {
template<class T> struct is_nothrow_default_constructible { static const bool value = false; };
template<class T> struct is_nothrow_copy_constructible { static const bool value = false; };
template<class T> struct is_nothrow_move_constructible { static const bool value = false; };
template<> struct is_nothrow_copy_constructible<int> { static const bool value = true; };
template<class T> struct char_traits { typedef long int_type; };
}
template<class F, class Arg> struct __is_nothrow_invocable { static const bool value = false; };
struct Callable { int operator()(int) const noexcept { return 0; } };
static_assert(!std::is_nothrow_default_constructible<Callable>::value, "primary body decides the value");
static_assert(!std::is_nothrow_copy_constructible<Callable>::value, "copy primary body");
static_assert(!std::is_nothrow_move_constructible<Callable>::value, "move primary body");
static_assert(std::is_nothrow_copy_constructible<int>::value, "explicit specialization body");
static_assert(!__is_nothrow_invocable<const Callable&, int>::value, "vendor primary body");
static_assert(__is_same(std::char_traits<char>::int_type, long), "character primary type alias");
int main() { return 0; }
