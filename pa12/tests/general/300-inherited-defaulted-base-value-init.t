void * operator new(unsigned long, void * pointer)
{
  return pointer;
}

struct base
{
  int * pointer;
  base() = default;
};

struct derived : base
{
  using base::base;
};

struct holder
{
  derived value;

  holder()
      : value()
  {
  }
};

int main()
{
  unsigned char storage[sizeof(holder)];
  for(unsigned long i = 0; i != sizeof(storage); ++i) {
    storage[i] = 0xff;
  }
  holder * object = new (storage) holder();
  return object->value.pointer == 0 ? 0 : 1;
}
