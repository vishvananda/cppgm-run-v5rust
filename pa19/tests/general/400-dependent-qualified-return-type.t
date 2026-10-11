// HHC-070
template<class... T>
struct tuple;

template<unsigned long I, class T>
struct tuple_element;

template<unsigned long I, class... T>
typename tuple_element<I, tuple<T...>>::type& get(tuple<T...>&) noexcept;
