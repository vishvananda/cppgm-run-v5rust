int select(char* const&)
{
  return 1;
}

int select(char*&&)
{
  return 2;
}

int main()
{
  return select(0L) == 2 ? 0 : 1;
}
