struct outer
{
  template<class T>
  struct allocator;

private:
  struct base {};
};

template<class T>
struct outer::allocator
{
  base *primary;
};

template<>
struct outer::allocator<void>
{
  base *specialized;
};

int main()
{
  return sizeof(outer::allocator<void>) == sizeof(void *) ? 0 : 1;
}
