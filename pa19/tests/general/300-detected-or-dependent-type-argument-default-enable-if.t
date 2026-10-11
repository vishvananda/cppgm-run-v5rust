template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class...>
struct make_void {
  typedef void type;
};

template<class... Ts>
using void_t = typename make_void<Ts...>::type;

template<class T>
struct iterator_traits {
  typedef int iterator_category;
  typedef long difference_type;
};

template<class T>
using iter_cat = typename T::iterator_category;

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

template<class T, class U>
struct is_convertible {
  static constexpr bool value = true;
};

template<class T, class U>
using has_iter_cat = is_convertible<detected_or_t<nat, iter_cat, iterator_traits<T> >, U>;

template<class RandIter, enable_if_t<has_iter_cat<RandIter, int>::value, int> = 0>
inline typename iterator_traits<RandIter>::difference_type distance_like(RandIter first,
                                                                         RandIter last) {
  return last - first;
}

int main() {
  return distance_like(0, 0);
}
