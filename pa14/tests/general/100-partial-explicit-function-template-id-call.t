template<class T, class U>
int f(T t, U u)
{
  return t + u;
}

template<class T, class U>
auto g(T t, U u) -> decltype(f<T>(t, u))
{
  return f<T>(t, u);
}

int main()
{
  return g(1, 2) != 3;
}
