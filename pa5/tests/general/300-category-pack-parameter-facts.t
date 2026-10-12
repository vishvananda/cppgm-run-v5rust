template<class... T> struct Wrapper {};
template<class... T> void unnamed_pack(T...);
template<class T> void unnamed_varargs(T...);
template<class... T> void consumed_pack(T..., Wrapper<T...>...);
template<class... T> void named_pack(T... values);
template<class... T> void nested_type_pack(decltype(T{})...);
