template<class R, class A>
struct result
{
  typedef R type;
};

template<class T, class... Args>
struct pred
{
};

template<class T, class... Args>
inline typename result<void, pred<T, Args...> >::type f(T *p, Args... args)
{
  (void)p;
}

int main()
{
  int x = 0;
  f(&x);
  return 0;
}
