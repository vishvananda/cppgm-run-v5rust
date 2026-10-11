// A saved dependent decltype can contain a pack expansion inside structured
// template-id syntax.  Replay must expand every bound element before applying
// the surrounding metafunctions.
template<class... T>
struct list {};

template<class... T>
struct context
{
  list<T...> value();
};

template<class T>
struct identity
{
  typedef T type;
};

template<class T>
struct unwrap;

template<class... T>
struct unwrap<list<T...> >
{
  typedef list<T...> type;
};

template<class... T>
typename unwrap<typename identity<
    decltype(((context<T...> *) 0)->value())
  >::type>::type * make(T && ...)
{
  return 0;
}

int main()
{
  return make(1, 2L) ? 1 : 0;
}
