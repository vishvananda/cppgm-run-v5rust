// N3485 14.6p10, 14.8.2: complete explicit heads retain the resolved function;
// namespace, global, static-member and non-call qualified names; expect exit 0
namespace ns {
template<class T> T make();
template<class T, class... U> T packed();
}
template<class T> T make();
struct C { template<class T> static T make(); };
template<class T> auto a(T t) -> decltype(ns::make<int>(), t) { return t; }
template<class T> auto b(T t) -> decltype(::make<int>(), t) { return t; }
template<class T> auto c(T t) -> decltype(C::make<int>(), t) { return t; }
template<class T> auto d(T t) -> decltype(ns::make<int>, t) { return t; }
template<class T> auto e(T t) -> decltype(ns::packed<int, long>(), t) { return t; }
int main() { return a(1) + b(2) + c(3) + d(4) + e(5) == 15 ? 0 : 1; }
