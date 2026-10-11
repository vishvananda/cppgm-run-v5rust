int seed()
{
  return 7;
}

struct pair
{
  int first;
  int second;
};

struct holder
{
  static pair values[2];
};

pair holder::values[2] = {{seed(), 1}, {2, 3}};

int main()
{
  return holder::values[0].first == 7 &&
         holder::values[0].second == 1 &&
         holder::values[1].first == 2 &&
         holder::values[1].second == 3 ? 0 : 1;
}
