union Invalid
{
  int first = 1;
  int second = 2;
};

int main()
{
  Invalid value;
  return value.first;
}
