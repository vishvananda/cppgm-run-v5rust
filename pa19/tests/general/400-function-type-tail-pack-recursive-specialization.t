template<class T, T V>
struct integral_constant
{
  static const T value = V;
};

typedef integral_constant<bool, true> true_type;
typedef integral_constant<bool, false> false_type;

template<bool B, class T, class F>
struct conditional
{
  typedef T type;
};

template<class T, class F>
struct conditional<false, T, F>
{
  typedef F type;
};

template<bool B, class T, class F>
using conditional_t = typename conditional<B, T, F>::type;

template<class A, class B>
struct is_same : false_type
{
};

template<class A>
struct is_same<A, A> : true_type
{
};

struct first
{
};

struct second
{
};

struct third
{
};

template<int I, class Props>
struct list;

template<int I, class Head, class... Tail>
struct list<I, void(Head, Tail...)>
{
  template<class T>
  struct find :
      conditional_t<
          is_same<T, Head>::value,
          true_type,
          typename list<I + 1, void(Tail...)>::template find<T> >
  {
  };
};

template<int I, class Head>
struct list<I, void(Head)>
{
  template<class T>
  struct find : is_same<T, Head>
  {
  };
};

using props = void(first, second, third);

int main()
{
  return list<0, props>::find<third>::value ? 0 : 1;
}
