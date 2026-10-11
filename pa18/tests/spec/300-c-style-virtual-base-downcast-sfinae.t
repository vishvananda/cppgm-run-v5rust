struct B {}; struct D : virtual B {};
template<class, class> constexpr bool probe(...) { return true; }
template<class T, class U, class = decltype((U *)((T *)0))> constexpr bool probe(int) { return false; }
static_assert(probe<B, D>(0), "");
