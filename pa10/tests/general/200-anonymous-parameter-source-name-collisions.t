int names(int, int t1)
{
  int __param0 = t1;
  int t2 = __param0 + 1;
  {
    int __param0 = t2;
    t2 = __param0 + 1;
  }
  return t2;
}

void unused(int)
{
  int __param0 = 3;
}

int main()
{
  return names(0, 3) - 5;
}
