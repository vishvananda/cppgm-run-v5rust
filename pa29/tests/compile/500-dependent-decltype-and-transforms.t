template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class T>
struct trait {
  static const bool value = true;
};

template<class T, class U = T>
using __enable_if_t = typename enable_if<trait<T>::value, U>::type;

template<class T>
using decay_t = __decay(T);

template<class T>
T&& declval();

template<class T>
using pointer_t [[gnu::nodebug]] = __add_pointer(T);

enum class Kind { A };

template<class T>
struct remove_all {
  using type [[gnu::nodebug]] = __remove_all_extents(T);
};

template<class T>
struct underlying {
  typedef __underlying_type(T) type;
};

template<class F, class A0, class... Args, class = int>
inline constexpr decltype((declval<A0>().*declval<F>())(declval<Args>()...))
invoke_probe(F&&, A0&&, Args&&...) noexcept(noexcept((declval<A0>().*declval<F>())(declval<Args>()...))) {
  return 0;
}

template<class T, __enable_if_t<T, int> = 0>
void only(T);

int main() {
  return 0;
}
