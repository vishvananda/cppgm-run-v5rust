typedef unsigned long size_t;

template<class... T>
struct tuple_types {};

template<size_t... I>
struct indices {};

template<class From>
struct copy_ref {
  template<class To>
  using apply = To;
};

template<class TupleTypes, class Indices>
struct make_flat;

template<class... Types, size_t... Idx>
struct make_flat<tuple_types<Types...>, indices<Idx...> > {
  template<class Tp>
  using apply_quals =
      tuple_types<typename copy_ref<Tp>::template apply<__type_pack_element<Idx, Types...> >...>;
};

struct S {};

int main()
{
  typedef typename make_flat<tuple_types<S&&>, indices<0> >::template apply_quals<S&&> selected;
  selected value;
  (void)value;
  return 0;
}
