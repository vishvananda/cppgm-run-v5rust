int first(int const (&a)[3])
{
  return a[0];
}

int pick(int const (&)[3])
{
  return 1;
}

int pick(int (&&a)[3])
{
  return a[0] == 1 ? 2 : 3;
}

int main()
{
  if(first({1, 2, 3}) != 1) {
    return 1;
  }
  return pick({1, 2, 3}) == 2 ? 0 : 2;
}
