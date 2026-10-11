struct Iter
{
  int * p;

  Iter(int * value) : p(value) {}

  Iter & operator++()
  {
    ++p;
    return *this;
  }
};

int main()
{
  int values[2] = {1, 2};
  Iter it(values);
  for(int i = 0; i != 1; ++i, ++it) {
  }
  return *it.p == 2 ? 0 : 1;
}
