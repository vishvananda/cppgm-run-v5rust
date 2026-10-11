struct allocator_tag
{
  int value;
};

template<class Alloc>
int join_alloc(Alloc & left, Alloc & right)
{
  left.value = right.value;
  return left.value;
}

template<class T>
struct allocator_base
{
  allocator_tag alloc;

  allocator_base() : alloc() { alloc.value = 7; }

  allocator_tag & get_alloc()
  {
    return alloc;
  }

  const allocator_tag & get_alloc() const
  {
    return alloc;
  }
};

template<class T>
struct store : allocator_base<T>
{
  int move_from(store & other)
  {
    return join_alloc(this->alloc, other.get_alloc());
  }
};

int main()
{
  store<int> first;
  store<int> second;
  second.alloc.value = 11;
  return first.move_from(second) == 11 ? 0 : 1;
}
