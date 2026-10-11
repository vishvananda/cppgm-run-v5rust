int first(int (&a)[3])
{
  return a[0];
}

int main()
{
  return first({1, 2, 3});
}
