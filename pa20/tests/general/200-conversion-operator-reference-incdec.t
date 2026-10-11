struct Counter
{
  unsigned value;

  Counter(unsigned initial) : value(initial) {}

  operator unsigned &()
  {
    return value;
  }
};

int main()
{
  Counter counter(3);
  ++counter;
  counter++;
  return counter.value == 5 ? 0 : 1;
}
