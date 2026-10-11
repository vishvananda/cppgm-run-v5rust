template<class T>
struct remove_cv {
  typedef T type;
};

template<class T>
using remove_cv_t = typename remove_cv<T>::type;

template<class T, bool = true>
struct make_unsigned;

template<class T>
struct make_unsigned<T, true> {
  typedef T type;
};

template<class T, class U>
struct copy_cv {
  typedef U type;
};

template<class T, class U>
using copy_cv_t = typename copy_cv<T, U>::type;

template<class T>
using make_unsigned_t = copy_cv_t<T, typename make_unsigned<remove_cv_t<T> >::type>;

template<class T>
constexpr make_unsigned_t<T> to_unsigned_like(T x) {
  return static_cast<make_unsigned_t<T> >(x);
}

template<class P>
struct pointer_traits {
  template<class T>
  struct rebind {
    typedef T type;
  };
};

template<class P, class T>
using rebind_pointer_t = typename pointer_traits<P>::template rebind<T>::type;

template<class P>
struct holder {
  typedef rebind_pointer_t<P, int> type;
};

int main() {
  return 0;
}
