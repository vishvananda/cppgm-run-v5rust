struct allocation
{
  static void *operator new(unsigned long);
};

template<class T>
struct box
{
  struct item : allocation
  {
  };
};

int main()
{
  box<int>::item *pointer = new box<int>::item;
  return pointer ? 0 : 1;
}
