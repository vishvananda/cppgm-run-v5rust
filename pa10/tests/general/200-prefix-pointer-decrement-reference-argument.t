int * touch(int *&value)
{
  return value;
}

int main()
{
  int values[3];
  int *p = values + 2;
  return touch(--p) == values + 1 ? 0 : 1;
}
