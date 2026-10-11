template<bool B>
struct bool_constant {
  static const bool value = B;
};

template<class A, class B>
struct is_same : bool_constant<false> {};

template<class A>
struct is_same<A, A> : bool_constant<true> {};

template<class...>
using void_t = void;

struct nat {};

template<class Default, class Void, template<class...> class Op, class... Args>
struct detector {
  typedef Default type;
};

template<class Default, template<class...> class Op, class... Args>
struct detector<Default, void_t<Op<Args...> >, Op, Args...> {
  typedef Op<Args...> type;
};

template<class Default, template<class...> class Op, class... Args>
using detected_or_t = typename detector<Default, void, Op, Args...>::type;

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class T>
struct iterator_traits {};

template<class T>
struct iterator_traits<T*> {
  typedef int iterator_category;
};

template<class T>
using iterator_category = typename T::iterator_category;

template<class T>
using has_iter_cat = is_same<detected_or_t<nat, iterator_category, iterator_traits<T> >, int>;

template<class T, enable_if_t<has_iter_cat<T>::value, int> = 0>
int distance_like(T, T) {
  return 0;
}

int main() {
  return distance_like((int*)0, (int*)0);
}
