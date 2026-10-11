namespace boost {
namespace container {
namespace dtl {

template<unsigned long...>
struct index_tuple
{
};

template<unsigned long N>
struct build_number_seq;

template<>
struct build_number_seq<1>
{
  typedef index_tuple<0> type;
};

template<class Allocator, class... Args>
struct proxy
{
  typedef typename build_number_seq<sizeof...(Args)>::type index_tuple_t;

  template<class Iterator>
  void call(Allocator & a, Iterator p)
  {
    this->priv(a, index_tuple_t(), p);
  }

private:
  template<unsigned long... IdxPack, class Iterator>
  void priv(Allocator &, const index_tuple<IdxPack...> &, Iterator)
  {
  }
};

int value;

}
}
}

int main()
{
  boost::container::dtl::proxy<int, int> * p = 0;
  int a = 0;
  p->call(a, &boost::container::dtl::value);
  return 0;
}
