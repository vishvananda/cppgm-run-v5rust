typedef decltype(sizeof(0)) size_t;

struct Holder {
  int value;

  void *operator new(size_t bytes, size_t extra);

  Holder() : value(7) {}
};

char storage[128];

void *Holder::operator new(size_t bytes, size_t extra)
{
  return storage + extra - extra + bytes - bytes;
}

int main()
{
  size_t extra = 5;
  Holder *p = new (extra) Holder();
  return p->value == 7 ? 0 : 1;
}
