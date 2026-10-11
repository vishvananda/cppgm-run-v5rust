// N3485 14.2/4: after a dependent nested-name-specifier a name without the
// template keyword is assumed not to name a template, so ns::Y<T>::value < 2
// is a comparison; expect exit 0
template <int N> struct I { static const int v = N; };
namespace ns { template <class T> struct Y { static const int value = 3; }; }
template <class T> struct A { static const int v = I<ns::Y<T>::value < 2>::v; };
int main() { return A<int>::v; }
