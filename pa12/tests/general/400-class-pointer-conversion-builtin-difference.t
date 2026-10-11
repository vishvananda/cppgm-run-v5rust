struct pointer
{
  int *value;
  operator int *() const { return value; }
};

int main()
{
  int value;
  pointer first = { &value }, last = { &value };
  return first - last;
}
