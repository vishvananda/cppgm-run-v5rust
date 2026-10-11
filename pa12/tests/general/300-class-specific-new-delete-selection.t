typedef decltype(sizeof(0)) size_t;

int selected;

struct Item
{
  static void *operator new(size_t size)
  {
    selected = 1;
    return ::operator new(size);
  }

  static void operator delete(void *storage) noexcept
  {
    selected = 2;
    ::operator delete(storage);
  }

  Item() noexcept {}
  ~Item() {}
};

int main()
{
  Item *value = new Item();
  delete value;
  if (selected != 2)
    return 1;

  selected = 0;
  Item *global_value = ::new Item();
  ::delete global_value;
  return selected;
}
