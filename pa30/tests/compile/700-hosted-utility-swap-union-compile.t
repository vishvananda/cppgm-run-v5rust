#include <utility>

class incomplete;

union erased_pointer
{
  void * object;
  const void * const_object;
  void (*function)();
  void (incomplete::*member)();
};

union erased_storage
{
  erased_pointer pointer;
  char bytes[sizeof(erased_pointer)];
};

void swap_storage(erased_storage & left, erased_storage & right)
{
  std::swap(left, right);
}

int main()
{
  erased_storage left;
  erased_storage right;
  left.pointer.object = 0;
  right.pointer.object = &left;
  swap_storage(left, right);
  return left.pointer.object == &left ? 0 : 1;
}
