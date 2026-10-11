template<bool>
struct gate;

template<>
struct gate<true> { typedef int type; };

template<class>
struct selected { static const bool value = false; };

namespace outer {
template<class>
struct selected { static const bool value = true; };

namespace component {
namespace detail {
template<class T>
using selected = ::selected<T>;
}
using namespace detail;

template<class T>
using allowed = typename gate<!selected<T>::value>::type;

template<class T, class = allowed<T> >
int f(T);
}
}

static_assert(sizeof(outer::component::f(0)) == sizeof(int), "");

int main() {}
// VALIDATION: compile-pass
// N3485 focus: 7.3.4 [namespace.udir], 14.5.7 [temp.alias]
