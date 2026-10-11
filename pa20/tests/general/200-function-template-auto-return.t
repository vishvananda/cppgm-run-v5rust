template<class T>
constexpr auto identity(T value)
{
  return value;
}

int main()
{
  return identity(0);
}
