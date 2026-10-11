// VALIDATION: an expression statement that starts with the injected class
// name and a parenthesized "*this" builds a temporary; it is not a
// declaration with a parenthesized declarator.

struct buffer
{
  int size;
  int capacity;

  buffer(int size, int capacity) : size(size), capacity(capacity) {}

  buffer(const buffer& other, int capacity)
    : size(other.size), capacity(capacity) {}

  void swap(buffer& other)
  {
    int size_copy = size;
    int capacity_copy = capacity;
    size = other.size;
    capacity = other.capacity;
    other.size = size_copy;
    other.capacity = capacity_copy;
  }

  void shrink_to_fit()
  {
    if (capacity > size)
      buffer(*this, size).swap(*this);
  }
};

int main()
{
  buffer object(3, 8);
  object.shrink_to_fit();
  if (object.size != 3) return 1;
  if (object.capacity != 3) return 2;
  return 0;
}
