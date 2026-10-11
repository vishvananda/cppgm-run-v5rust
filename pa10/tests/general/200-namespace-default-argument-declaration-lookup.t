namespace library
{
int default_value = 7;

int read(int value = default_value)
{
  return value;
}
}

int default_value = 99;

int main()
{
  return library::read() - 7;
}
