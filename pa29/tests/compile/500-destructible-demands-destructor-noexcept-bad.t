// Plain destructibility also queries the existing destructor's noexcept clause.
template<class T> struct X {
  ~X() noexcept(sizeof(typename T::missing) > 0);
};
static_assert(__is_destructible(X<int>), "destruction");
