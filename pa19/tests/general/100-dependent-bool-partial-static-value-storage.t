namespace mpl_
{
  template<bool B>
  struct bool_;

  template<class I, I val>
  struct integral_c;

  struct integral_c_tag;
}

namespace boost
{
  namespace mpl
  {
    using ::mpl_::bool_;
    using ::mpl_::integral_c;
    using ::mpl_::integral_c_tag;
  }

  template<class T, T val>
  struct integral_constant
  {
    typedef mpl::integral_c_tag tag;
    typedef T value_type;
    typedef integral_constant<T, val> type;
    static const T value = val;

    operator const mpl::integral_c<T, val>& () const
    {
      static const char data[sizeof(long)] = {0};
      const void* const pdata = data;
      return *static_cast<const mpl::integral_c<T, val>*>(pdata);
    }
  };

  template<class T, T val>
  T const integral_constant<T, val>::value;

  template<bool val>
  struct integral_constant<bool, val>
  {
    typedef mpl::integral_c_tag tag;
    typedef bool value_type;
    typedef integral_constant<bool, val> type;
    static const bool value = val;

    operator const mpl::bool_<val>& () const
    {
      static const char data[sizeof(long)] = {0};
      const void* const pdata = data;
      return *static_cast<const mpl::bool_<val>*>(pdata);
    }
  };

  template<bool val>
  bool const integral_constant<bool, val>::value;

  namespace detail
  {
    template<class T>
    struct alignment_of_impl
    {
      static const unsigned value = sizeof(T);
    };
  }

  template<class T>
  struct alignment_of
    : integral_constant<unsigned, detail::alignment_of_impl<T>::value>
  {
  };
}

template<class T>
int align_value(T)
{
  return boost::alignment_of<T>::value;
}

int main()
{
  return align_value(0) == sizeof(int) ? 0 : 1;
}
