template<class... Args>
int g(Args... args)
{
  auto f = [](Args... args2) { return sizeof...(args2); };
  return f(args...);
}

int main()
{
  return g(1, 'a') != 2;
}
