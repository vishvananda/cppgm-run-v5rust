struct item
{
  int value;

  explicit item(int v = 7) : value(v)
  {
  }
};

int main()
{
  item *items = new item[2];
  int ok = items[0].value == 7 && items[1].value == 7;
  delete[] items;
  return ok ? 0 : 1;
}
