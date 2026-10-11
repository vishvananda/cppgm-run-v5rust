template <class T>
struct allocation_result
{
  T * ptr;
  unsigned long count;
};

template <class T>
allocation_result<T> allocate_at_least(T * ptr, unsigned long n)
{
  return {ptr, n};
}

int main()
{
  int value = 7;
  allocation_result<int> result = allocate_at_least(&value, 1);
  return result.ptr == &value && result.count == 1 ? 0 : 1;
}
