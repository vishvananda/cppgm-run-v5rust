template<class T> struct outer {
  template<bool> struct inner {
    struct unused { static_assert(sizeof(T) == 0, "unused"); };
  };
};

outer<int>::inner<true> value;
