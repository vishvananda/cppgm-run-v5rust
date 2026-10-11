namespace host_alloc {
struct Tag {
  int marker;
};

char storage[16];
}

void *operator new(unsigned long, const host_alloc::Tag &tag)
{
  return tag.marker == 17 ? static_cast<void *>(host_alloc::storage) : 0;
}

void operator delete(void *p, const host_alloc::Tag &tag)
{
  if (p == static_cast<void *>(host_alloc::storage) && tag.marker == 17)
    host_alloc::storage[0] = 42;
}
