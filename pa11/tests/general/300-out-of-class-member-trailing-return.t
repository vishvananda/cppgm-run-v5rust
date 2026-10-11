// VALIDATION: compile-pass

struct array
{
  typedef int * iterator;

  int value;

  iterator begin() noexcept;
};

auto array::begin() noexcept -> iterator
{
  return &value;
}

int main()
{
  array a;
  a.value = 7;
  return *a.begin() == 7 ? 0 : 1;
}
