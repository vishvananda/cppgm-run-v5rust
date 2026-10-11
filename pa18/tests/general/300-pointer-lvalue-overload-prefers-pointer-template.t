template<class T>
struct traits;

template<class T>
struct traits<T *>
{
  typedef T element_type;
};

template<class T>
T *address(T *pointer)
{
  return pointer;
}

template<class Pointer>
typename traits<Pointer>::element_type *address(const Pointer &pointer)
{
  return pointer.operator->();
}

int main()
{
  int value = 0;
  int *pointer = &value;
  return address(pointer) == &value ? 0 : 1;
}
