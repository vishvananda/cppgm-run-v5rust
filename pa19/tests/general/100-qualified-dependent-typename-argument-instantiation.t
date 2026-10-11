namespace boost
{
  template<class T, T Value>
  struct integral_constant
  {
    static const T value = Value;
  };

  template<class T>
  struct remove_reference
  {
    typedef T type;
  };

  template<class T>
  struct remove_reference<T&>
  {
    typedef T type;
  };

  template<class T>
  struct is_function : integral_constant<bool, false>
  {
  };

  template<class T>
  struct is_complete
    : integral_constant<
        bool,
        is_function<typename boost::remove_reference<T>::type>::value || true>
  {
  };

  template<class From, class To>
  struct is_convertible : integral_constant<bool, true>
  {
    static_assert(boost::is_complete<To>::value, "complete");
    static_assert(boost::is_complete<From>::value, "complete");
  };
}

struct tag
{
};

static_assert(boost::is_convertible<tag, const tag&>::value, "ok");

int main()
{
  return 0;
}
