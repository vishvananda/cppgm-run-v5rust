int square(int x)
{
  return x * x;
}

template<typename R, typename A>
R apply(R (&fn)(A), A value)
{
  return fn(value);
}

int main()
{
  return apply(square, 3) == 9 ? 0 : 1;
}
