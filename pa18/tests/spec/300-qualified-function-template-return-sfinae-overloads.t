// N3485 focus: 14.8.2 [temp.deduct] substitution into a function-template
// return type participates in candidate selection for a qualified call.

namespace N {
namespace detail {

template<bool B, class T>
struct addrof_if {
};

template<class T>
struct addrof_if<true, T> {
  typedef T* type;
};

template<class T>
struct use_direct_address {
  static constexpr bool value = true;
};

template<class T>
typename addrof_if<!use_direct_address<T>::value, T>::type
addressof(T& value)
{
  return reinterpret_cast<T*>(&reinterpret_cast<char const volatile&>(value));
}

template<class T>
typename addrof_if<use_direct_address<T>::value, T>::type
addressof(T& value)
{
  return &value;
}

}

template<class T>
T* addressof(T& value)
{
  return N::detail::addressof(value);
}

}

struct object {
  int value;
};

int main()
{
  object value = { 0 };
  return N::addressof(value) == &value ? 0 : 1;
}
