template<class T> struct outer {
  template<class U> struct inner {};
};

template<class T> struct wrap {};

template<class T> struct holder {
  using type = wrap<typename outer<T>::template inner<long> >;
};
