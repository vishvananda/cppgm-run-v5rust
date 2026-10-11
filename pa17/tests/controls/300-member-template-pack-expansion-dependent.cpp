// N3485 14.5.3p5-p6, 14.6.2.1, 14.7.1p1: in a member template of zip<int,
// char>, list<pair<typename T::type, U>...> stays dependent on U (the bound
// pack T expands with U); instantiating zip<int, char> must not substitute
// into it; expect exit 0
template<class... T> struct list {};
template<class A, class B> struct pair {};
template<class... T> struct zip {
  template<class... U> static list<pair<T, U>...> with(list<U...>) { return list<pair<T, U>...>(); }
  template<class... U> static list<pair<typename T::type, U>...> member(list<U...>);
};
int main() { zip<int, char>::with(list<long, short>()); return 0; }
