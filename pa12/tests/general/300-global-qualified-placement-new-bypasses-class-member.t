typedef decltype(sizeof(0)) size_t;

void *operator new(size_t, void *address)
{
  return address;
}

struct Holder
{
  int value;

  void *operator new(size_t);

  Holder(int initial) : value(initial) {}
};

char storage[128];

int main()
{
  Holder *p = ::new ((void *)storage) Holder(7);
  return p->value == 7 ? 0 : 1;
}
