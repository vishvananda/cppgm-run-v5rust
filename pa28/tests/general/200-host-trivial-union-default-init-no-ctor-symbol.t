# host trivial union default init does not emit constructor symbols
# split
union HostTrivialUnion {
  unsigned char data[8];
};

struct HostUnionHolder {
  HostTrivialUnion storage;
  void *ptr;

  HostUnionHolder() : ptr(0) {}
};

int main()
{
  HostUnionHolder holder;
  return holder.ptr == 0 ? 0 : 1;
}
