namespace mpl {
template <class T, T val>
struct integral_c {
};
}

namespace boost {
template <class T, T val>
struct integral_constant {
  static const T value = val;

  operator const mpl::integral_c<T, val>& () const
  {
    static const char data[sizeof(long)] = {0};
    const void* const pdata = data;
    return *static_cast<const mpl::integral_c<T, val>*>(pdata);
  }

  operator T() const
  {
    return val;
  }

  T operator()() const
  {
    return val;
  }
};

template <class T, T val>
T const integral_constant<T, val>::value;
}

boost::integral_constant<int, 7> value;
