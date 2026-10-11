// HHC-127
template<class P>
struct X {
  static decltype(declval<const P&>().operator->()) call(const P& p) noexcept { return p.operator->(); }
};
