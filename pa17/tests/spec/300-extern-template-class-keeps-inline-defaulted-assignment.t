// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
struct empty_allocator
{
  empty_allocator & operator=(const empty_allocator &) = default;
};

extern template struct empty_allocator<char>;

void assign(empty_allocator<char> & target,
            const empty_allocator<char> & source)
{
  target = source;
}

int main()
{
  empty_allocator<char> first;
  empty_allocator<char> second;
  assign(first, second);
  return 0;
}
