struct item
{
  int value;
};

int main()
{
  item object;
  item & ref = (0, object);
  return &object == &ref ? 0 : 1;
}
