namespace first {

template<class T>
struct iterator_traits
{
  typedef int value_type;
};

template<class T>
struct remove_cv
{
  typedef T type;
};

template<class A, class B>
struct is_same
{
  static const bool value = false;
};

template<class A>
struct is_same<A, A>
{
  static const bool value = true;
};

template<bool V>
struct flag
{
  static const bool value = V;
};

}

namespace second {

template<class T>
struct iterator_traits: first::iterator_traits<T>
{
};

template<>
struct iterator_traits<void*>
{
};

template<class T>
T&& declval();

template<class T, class It>
first::flag<
    !first::is_same<typename first::remove_cv<T>::type,
                    typename iterator_traits<It>::value_type>::value>
range_check(It, It);

template<class T>
decltype(range_check<T>(declval<T const&>().begin(),
                        declval<T const&>().end()))
is_range(int);

template<class T>
char is_range(...);

}

struct X
{
  void* begin() const;
  void* end() const;
};

template<class A, class B>
struct same
{
  static const bool value = false;
};

template<class A>
struct same<A, A>
{
  static const bool value = true;
};

static_assert(same<decltype(second::is_range<X>(0)), char>::value,
              "missing member type must remove the candidate");

int main()
{
  return 0;
}
