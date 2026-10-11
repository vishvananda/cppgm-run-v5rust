int range_alive = 0;
int range_destroyed = 0;

struct Range
{
  int values[2];

  Range()
  {
    range_alive = 1;
    values[0] = 2;
    values[1] = 3;
  }

  ~Range()
  {
    range_alive = 0;
    ++range_destroyed;
  }

  int* begin() noexcept
  {
    return values;
  }

  int* end() noexcept
  {
    return values + 2;
  }
};

int main()
{
  int sum = 0;
  for(int value : Range()) {
    if(!range_alive || range_destroyed != 0) {
      return 1;
    }
    sum += value;
  }
  return sum == 5 && range_alive == 0 && range_destroyed == 1 ? 0 : 2;
}
