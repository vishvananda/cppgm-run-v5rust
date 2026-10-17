// N3485 14.5.3p7: bound packs in an expansion have a common length;
// expect reject
template<class... T>
struct list {};

template<class A, class B>
struct pair {};

template<class... A>
struct outer {
  template<class... B>
  struct inner {
    typedef list<pair<A, B>...> type;
  };
};

outer<int, long>::inner<char> object;
