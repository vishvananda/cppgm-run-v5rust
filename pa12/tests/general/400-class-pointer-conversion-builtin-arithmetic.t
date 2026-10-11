struct pointer
{
  int *value;
  operator int *() const { return value; }
};

int main()
{
  int values[2] = { 1, 2 };
  pointer p = { values };
  return *(p + 1) == 2 ? 0 : 1;
}
