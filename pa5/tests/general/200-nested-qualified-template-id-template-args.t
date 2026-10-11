namespace ns {
  template<class T> struct box {};
  template<class A, class B> struct andx {};
}

template<typename T>
using left_alias = ns::andx<ns::box<T>, int>;

template<typename T>
using right_alias = ns::andx<int, ns::box<T>>;
