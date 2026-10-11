// HHC-132
template<class _Iterator>
struct __bounded_iter {
  template <class _It>
  friend constexpr __bounded_iter<_It> __make_bounded_iter(_It, _It, _It);
};
