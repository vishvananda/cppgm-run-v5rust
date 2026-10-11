template<class... Tail>
struct callable
{
  template<class U>
  int operator()(U&& value, Tail... tail) const
  {
    return value;
  }
};

int main()
{
  int value = 7;
  callable<> f;
  return f(value) - 7;
}
