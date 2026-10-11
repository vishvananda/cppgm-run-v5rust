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

iterator saved_begin = { 0 };

iterator const& begin(range& value)
{
  saved_begin.current = value.values;
  return saved_begin;
}

iterator end(range& value)
{
  iterator result = { value.values + 2 };
  return result;
}
}

int main()
{
  model::range values = {{ 5, 6 }};
  int total = 0;
  for(int value : values) {
    total += value;
  }
  return total == 11 ? 0 : 1;
}
