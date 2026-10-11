// N3485 15.4p15: a deallocation function with no exception-specification is
// treated as noexcept(true), so a redeclaration without one agrees with the
// implicit declaration, and a new-expression whose constructor may throw
// frees the storage through a class deallocation function without having to
// guard that call.
typedef unsigned long size_type;

void operator delete(void*);

int released = 0;

struct Guarded
{
  int value;
  Guarded(int v) : value(v) { if (v < 0) throw v; }
  static void* operator new(size_type size);
  static void operator delete(void* storage, size_type size);
};

char storage[16];

void* Guarded::operator new(size_type) { return storage; }
void Guarded::operator delete(void*, size_type) { ++released; }

int make(int v)
{
  Guarded* object = new Guarded(v);
  const int result = object->value;
  delete object;
  return result;
}

int main()
{
  return make(4) == 4 && released == 1 ? 0 : 1;
}
