struct Iter
{
  int *p;

  int & operator*()
  {
    return *p;
  }

  Iter & operator++()
  {
    ++p;
    return *this;
  }

  bool operator!=(Iter other)
  {
    return p != other.p;
  }
};

struct Range
{
  int values[2];

  Iter begin()
  {
    Iter it;
    it.p = values;
    return it;
  }

  Iter end()
  {
    Iter it;
    it.p = values + 2;
    return it;
  }
};

int main()
{
  Range range;
  range.values[0] = 7;
  range.values[1] = 9;
  auto first = [](Range &r) {
    for(auto value : r) {
      return value;
    }
    return 0;
  };
  return first(range) == 7 ? 0 : 1;
}
