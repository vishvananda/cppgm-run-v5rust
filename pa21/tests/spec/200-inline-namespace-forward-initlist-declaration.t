// VALIDATION: compile-pass
// N3485 focus: 14.6.1 [temp.local], 14.8 [temp.fct.spec]

namespace std {
inline namespace __1 {
template<class T>
T && __declval(int);

template<class T>
T __declval(long);

template<class T>
decltype(std::__1::__declval<T>(0)) declval() noexcept;
}

template<class E>
class initializer_list;

template<class E>
const E * begin(initializer_list<E>);
}

int main()
{
  return 0;
}
