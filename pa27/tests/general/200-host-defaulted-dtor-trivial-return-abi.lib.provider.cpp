struct Result {
  bool first;
  unsigned long second;

  Result(bool first_in = false, unsigned long second_in = 0)
      : first(first_in), second(second_in) {}
  ~Result() = default;
};

Result host_choose(unsigned long seed)
{
  return Result(seed != 0, seed + 1);
}
