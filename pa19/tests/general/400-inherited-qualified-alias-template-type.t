template<class T>
struct BaseTraits {
  typedef T value_type;

  template<class U>
  using rebind_traits = BaseTraits<U>;
};

template<class T>
struct DerivedTraits : BaseTraits<T> {
};

template<class T>
struct GetValue {
  typedef T type;
};

template<class Node>
struct Holder {
  typedef DerivedTraits<Node> node_traits;
  typedef typename node_traits::template rebind_traits<
      typename GetValue<Node>::type> value_traits;
};

int main()
{
  Holder<int>::value_traits::value_type x = 7;
  return x == 7 ? 0 : 1;
}
