template<class T>
struct remove_cv {
  typedef T type;
};

template<class T>
using remove_cv_t = typename remove_cv<T>::type;

template<class T, bool = true>
struct make_unsigned {};

template<>
struct make_unsigned<long, true> {
  typedef unsigned long type;
};

template<class From>
struct copy_cv {
  template<class To>
  using apply = To;
};

template<class From, class To>
using copy_cv_t = typename copy_cv<From>::template apply<To>;

template<class T>
using make_unsigned_t = copy_cv_t<T, typename make_unsigned<remove_cv_t<T> >::type>;

template<class T>
struct wrapper {
  typedef make_unsigned_t<T> type;
};

int main() {
  wrapper<long>::type value = 5;
  return value == 5 ? 0 : 1;
}
