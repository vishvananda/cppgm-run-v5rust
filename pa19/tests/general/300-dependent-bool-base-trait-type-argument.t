namespace std {
  template<class T, T V>
  struct integral_constant {
    static const T value = V;
    constexpr operator T() const { return value; }
  };

  typedef integral_constant<bool, true> true_type;
  typedef integral_constant<bool, false> false_type;

  template<bool B, class T = void>
  struct enable_if {};

  template<class T>
  struct enable_if<true, T> {
    typedef T type;
  };

  template<bool B, class T = void>
  using __enable_if_t = typename enable_if<B, T>::type;

  template<class T>
  struct is_default_constructible : true_type {};

  namespace __detail {
    struct _Hash_node_base {};

    template<typename T, typename...>
    using __first_t = T;

    template<typename... Bn>
    __first_t<true_type, __enable_if_t<bool(Bn::value)>...> __and_fn(int);

    template<typename... Bn>
    false_type __and_fn(...);
  }

  template<typename... Bn>
  struct __and_ : decltype(__detail::__and_fn<Bn...>(0)) {};

  struct _Enable_default_constructor_tag {};

  template<bool Switch, class TagT = void>
  struct _Enable_default_constructor {
    _Enable_default_constructor() {}
    explicit _Enable_default_constructor(_Enable_default_constructor_tag) {}
  };

  template<class TagT>
  struct _Enable_default_constructor<false, TagT> {
    _Enable_default_constructor() {}
    explicit _Enable_default_constructor(_Enable_default_constructor_tag) {}
  };

  template<class Equal, class Hash, class Alloc>
  using _Hashtable_enable_default_ctor =
      _Enable_default_constructor<__and_<
                                    is_default_constructible<Equal>,
                                    is_default_constructible<Hash>,
                                    is_default_constructible<Alloc> >{},
                                  __detail::_Hash_node_base>;

  template<class Key, class Equal, class Hash, class Alloc>
  struct _Hashtable : _Hashtable_enable_default_ctor<Equal, Hash, Alloc> {
    _Hashtable()
      : _Hashtable_enable_default_ctor<Equal, Hash, Alloc>(
            _Enable_default_constructor_tag()) {}
  };
}

template<class X>
struct Uses : std::_Hashtable<X, X, X, X> {
  Uses() {}
};

int main()
{
  Uses<int> table;
  return 0;
}
