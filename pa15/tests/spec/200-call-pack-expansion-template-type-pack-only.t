// VALIDATION: compile-pass
// N3485 focus: 14.5.3 [temp.variadic] pack expansions

int sum(int a, int b)
{
  return a + b;
}

template<class... Ts>
struct sizes
{
  int run()
  {
    return sum(sizeof(Ts)...);
  }
};

int main()
{
  sizes<char, short> value;
  return value.run() == 3 ? 0 : 1;
}
