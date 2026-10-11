enum token
{
  first,
  second,
  third
};

struct level
{
  token operators[4];
  int count;
};

int main()
{
  level levels[] = {
    {{first}, 1},
    {{second, third}, 2},
    {}
  };

  return levels[0].operators[0] == first &&
         levels[0].operators[3] == first &&
         levels[1].operators[0] == second &&
         levels[1].operators[1] == third &&
         levels[1].operators[3] == first &&
         levels[2].operators[0] == first &&
         levels[2].count == 0 ? 0 : 1;
}
