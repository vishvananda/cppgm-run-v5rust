struct X {};

template<class T>
auto value(T t) -> decltype(static_cast<const X>(t))
{
  return t;
}

int main()
{
  value(X{});
  return 0;
}
