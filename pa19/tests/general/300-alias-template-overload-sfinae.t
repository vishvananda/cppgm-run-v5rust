template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class T>
struct decay
{
  typedef T type;
};

template<class A, class B>
struct is_same
{
  static const bool value = false;
};

template<class A>
struct is_same<A, A>
{
  static const bool value = true;
};

template<class T>
struct remove_cvref
{
  typedef T type;
};

template<class T>
struct remove_cvref<const T &>
{
  typedef T type;
};

template<class Sig>
struct function;

template<class R>
struct function<R()>
{
  template<class Func,
           bool Self =
               is_same<typename remove_cvref<Func>::type, function>::value>
  using decay_t = typename enable_if_t<!Self, decay<Func> >::type;

  template<class Func>
  struct callable
  {
    static const bool value = true;
  };

  template<class Cond, class Tp = void>
  using requires_t = enable_if_t<Cond::value, Tp>;

  function & operator=(const function &)
  {
    return *this;
  }

  template<class Func>
  requires_t<callable<decay_t<Func> >, function &>
  operator=(Func &&)
  {
    return *this;
  }
};

void test(const function<int()> & src)
{
  function<int()> dst;
  dst = src;
}

int main()
{
  return 0;
}
