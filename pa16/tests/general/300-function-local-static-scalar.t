int next_id()
{
  static int id = 0;
  id = id + 1;
  return id;
}

int main()
{
  int a = next_id();
  int b = next_id();
  return (a == 1 && b == 2) ? 0 : 7;
}
