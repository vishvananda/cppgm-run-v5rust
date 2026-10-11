// VALIDATION: compile-pass
// A lazy alias must recompute a hosted builtin base-class trait after its
// dependent pack has been substituted.

namespace std {
template<class B, class D>
struct is_base_of { static const bool value = __is_base_of(B, D); };
}

namespace boost { namespace mp11 {
template<bool B> struct mp_bool { static const bool value = B; };
using mp_true = mp_bool<true>;
using mp_false = mp_bool<false>;
template<class T> using mp_to_bool = mp_bool<T::value>;
template<class T> struct id {};
template<class... T> struct inherit : T... {};
template<class... T>
using contains =
    mp_to_bool<std::is_base_of<id<void>, inherit<id<T>...> > >;
}}

static_assert(boost::mp11::contains<void>::value, "");

int main() { return 0; }
