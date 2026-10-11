// VALIDATION: compile-pass

struct iterator
{
  int* value;

  operator const int*() const { return value; }
  iterator operator-(long) const { return *this; }
};

long distance(iterator first, int* last)
{
  return first - last;
}

int main()
{
  int values[1];
  iterator first;
  first.value = values;
  return distance(first, values) == 0 ? 0 : 1;
}
