// HHC-117
template<class T> struct integral_constant { static const bool value = false; };
template<class Iter> struct __libcpp_is_contiguous_iterator : integral_constant<Iter> {};
template<class Iter> struct X { static_assert(__libcpp_is_contiguous_iterator<Iter>::value, ""); };
