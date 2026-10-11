namespace std { inline namespace __1 {
template<class A, class B> struct is_same { static constexpr bool value = false; };
template<class A> struct is_same<A, A> { static constexpr bool value = true; };
template<class T> struct char_traits { typedef T char_type; };
}}

template<class _CharT, class _Traits>
struct X {
  using traits_type = _Traits;
  static_assert(std::__1::is_same<_CharT, typename traits_type::char_type>::value, "");
};

typedef X<char, std::__1::char_traits<char>> Y;
