int *assign_address(int &value, int next)
{
  return &(value = next);
}

int main()
{
  int value = 1;
  int *p = assign_address(value, 2);
  *p = 3;
  return value == 3 ? 0 : 1;
}
