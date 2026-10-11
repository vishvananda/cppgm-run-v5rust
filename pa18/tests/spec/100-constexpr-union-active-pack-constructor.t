// N3485 focus: 5.19 [expr.const], 9.5 [class.union], 14.8.2 [temp.deduct]
struct tag {};
template<class T> constexpr T&& forward(T& value) {
  return static_cast<T&&>(value);
}
template<class T> union U {
  unsigned char dummy;
  T value;
  constexpr U(tag) : dummy() {}
  template<class... A> constexpr U(A&&... args) : value(forward<A>(args)...) {}
};
constexpr U<int> inactive = U<int>(tag());
constexpr U<int> active = U<int>(1);
static_assert(inactive.dummy == 0, "inactive union member is not evaluated");
static_assert(active.value == 1, "pack-expanded member initializer");
int main() {}
