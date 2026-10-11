// VALIDATION: compile-pass
// A defaulted non-type argument also carries its value type; substituting a
// dependent member result must keep the argument classified as a value.

namespace lib {
template<class> struct trait { static bool const value = false; };
template<class Arg, bool = trait<int>::value>
struct result { typedef Arg type; };
}

template<class T> struct identity { typedef T type; };
template<class> struct wrapper {
  template<class T>
  typename lib::result<identity<typename identity<T>::type> >::type f(T&);
};

int main() { wrapper<int> w; int value; w.f(value); }
