template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<unsigned long I, class... Types>
struct tuple_element;

template<class Head, class... Tail>
struct tuple_element<0, Head, Tail...> {
  typedef Head type;
};

template<unsigned long I, class Head, class... Tail>
struct tuple_element<I, Head, Tail...> : tuple_element<I - 1, Tail...> {};

template<class... Types>
struct tuple {};

template<unsigned long I, class... Types>
const typename tuple_element<I, Types...>::type& get(const tuple<Types...>&) {
  static typename tuple_element<I, Types...>::type value{};
  return value;
}

template<unsigned long I, class... Types>
enable_if_t<(I >= sizeof...(Types))> get(const tuple<Types...>&) = delete;

int main() {
  const tuple<int, char> object{};
  const tuple<int, char>* p = &object;
  (void)get<0>(*p);
  return 0;
}
