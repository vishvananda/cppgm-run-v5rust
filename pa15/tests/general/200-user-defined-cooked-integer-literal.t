int operator ""_add(unsigned long long value)
{
  return (int)value + 7;
}

int main()
{
  return 35_add == 42 ? 0 : 1;
}
