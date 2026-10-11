template <bool, class T = void>
struct enable_if {};

template <class T>
struct enable_if<true, T> {
  typedef T type;
};

template <class Cond, class T = void>
using require = typename enable_if<Cond::value, T>::type;

template <class T>
struct is_callable {
  static const bool value = false;
};

struct callable {};

template <>
struct is_callable<callable> {
  static const bool value = true;
};

struct function_like {
  template <class F, class = require<is_callable<F> > >
  function_like(F) {}
};

int main()
{
  callable c;
  function_like f(c);
  (void)f;
}
