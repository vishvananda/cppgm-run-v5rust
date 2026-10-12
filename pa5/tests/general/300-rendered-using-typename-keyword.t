template<class T> void g() noexcept([] {
  using typename T::V;
  V value;
  return true;
}());
