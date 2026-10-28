// A soft deleted-destructor event still demands its existing noexcept clause.
template<class T> struct X {
  X(int);
  ~X() noexcept(sizeof(typename T::missing) > 0) = delete;
};
static_assert(!__is_convertible(int, X<int>), "conversion");
