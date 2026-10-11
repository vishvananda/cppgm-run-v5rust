struct L
{
  int value;
  static int global(L const & item);
};

int L::global(L const & item)
{
  return item.value + 1;
}
