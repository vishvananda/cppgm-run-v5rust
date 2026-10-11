// recast of hosted pair-vector-arg-ranges link smoke: compile-time check of
// inline-namespace class templates + aggregate member access (host symbol
// naming covered by pa26 300-user-inline-namespace-substitution).
namespace helper_inline_ns { inline namespace v1 {
template<class A, class B> struct pair { A first; B second; };
template<class T> struct vec { T value; };
}}
constexpr bool arg_ranges_ok() {
  helper_inline_ns::v1::vec<helper_inline_ns::v1::pair<unsigned long, unsigned long> > r{ {1, 2} };
  return r.value.first == 1 && r.value.second == 2;
}
static_assert(arg_ranges_ok(), "");
int main(){return 0;}
