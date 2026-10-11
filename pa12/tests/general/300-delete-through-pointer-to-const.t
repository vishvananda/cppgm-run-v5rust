int destroyed;

struct object {
  ~object()
  {
    destroyed = 1;
  }
};

int main()
{
  const object * p = new object;
  delete p;
  return destroyed == 1 ? 0 : 1;
}
