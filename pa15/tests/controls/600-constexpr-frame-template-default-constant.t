// Class completion during a nested constexpr call evaluates each template
// default in its own fact context, despite the live outer frame. expect exit 0
template<bool> struct flag {};
template<class T> struct metadata {
  template<bool B = true> static flag<B> check();
  typedef decltype(check()) type;
  static const bool value = true;
};
template<class T> void target() noexcept(metadata<T>::value);
template<class T> constexpr bool leaf() { return noexcept(target<T>()); }
template<class T> constexpr bool middle() { return leaf<T>(); }
static_assert(middle<int>(), "nested constant frame");
int main() { return 0; }
