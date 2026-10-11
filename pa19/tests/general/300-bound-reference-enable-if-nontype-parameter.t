struct true_type {
  static const bool value = true;
};

template <bool, class T = void>
struct enable_if {};

template <class T>
struct enable_if<true, T> {
  typedef T type;
};

template <bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template <class...>
struct all : true_type {};

template <class S, class T, class = void>
struct probe : true_type {};

template <class S,
          class T,
          enable_if_t<all<probe<S &, const T &> >::value, int> = 0>
int call(S &&, const T &)
{
  return 7;
}

struct stream {};
struct value {};

int main()
{
  stream out;
  value x;
  return call(out, x) == 7 ? 0 : 1;
}
