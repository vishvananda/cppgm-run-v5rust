template <class T>
struct allocation_result
{
  T * ptr;
  unsigned long count;

  allocation_result(T * __ptr, unsigned long __count) : ptr(__ptr), count(__count) {}
};

template <class T>
allocation_result<T> allocate_at_least(T * ptr, unsigned long n)
{
  return allocation_result<T>(ptr, n);
}

struct Node
{
  int value;
};

int main()
{
  int value = 7;
  Node node = {9};
  allocation_result<int> ints = allocate_at_least(&value, 1);
  allocation_result<Node> nodes = allocate_at_least(&node, 1);
  return ints.ptr == &value &&
         ints.count == 1 &&
         nodes.ptr == &node &&
         nodes.ptr->value == 9 &&
         nodes.count == 1 ? 0 : 1;
}
