struct X
{
  operator int() { return 1; }
};

template<class T, decltype(T().operator int()) N>
int value()
{
  return N;
}

int main()
{
  return value<X, 3>() - 3;
}
