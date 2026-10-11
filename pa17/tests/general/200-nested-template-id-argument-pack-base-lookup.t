namespace meta {

template<class T>
struct identity
{
};

template<class... T>
struct inherit : T...
{
};

template<class T>
struct holder
{
  using type = T;
};

template<class T>
using select = typename holder<T>::type;

template<class... T>
struct make
{
  using type = select<inherit<identity<T>...>>;
};

} // namespace meta

struct name
{
};

using result = meta::make<name>::type;

int main()
{
  return sizeof(result) > 0 ? 0 : 1;
}
