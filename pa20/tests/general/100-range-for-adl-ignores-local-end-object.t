namespace model
{
struct iterator
{
  int * current;

  int operator*() const
  {
    return *current;
  }

  iterator& operator++()
  {
    ++current;
    return *this;
  }
};

bool operator!=(iterator left, iterator right)
{
  return left.current != right.current;
}

struct range
{
  int values[2];
};

iterator begin(range& value)
{
  iterator result = { value.values };
  return result;
}

iterator end(range& value)
{
  iterator result = { value.values + 2 };
  return result;
}
}

int main()
{
  model::range values = {{ 3, 4 }};
  int end = 0;
  int total = 0;
  for(int value : values) {
    total += value;
  }
  return total == 7 && end == 0 ? 0 : 1;
}
