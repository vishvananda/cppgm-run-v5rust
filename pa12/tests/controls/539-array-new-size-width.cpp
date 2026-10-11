// The size an array new-expression requests is a std::size_t (N3485
// 5.3.4p10): a runtime count widens by its promoted type's signedness before
// it is scaled and the cookie is added, and a constant request is not
// truncated to the count's type. 2^28 16-byte elements need 4 GiB.
typedef unsigned long size_type;

size_type requested = 0;

void* operator new[](size_type size) {
  static char storage[64];
  requested = size;
  return storage;
}

void operator delete[](void*) noexcept {}

struct Destructible {
  long first;
  long second;
  ~Destructible() {}
};

unsigned unsigned_count() { return 1u << 28; }
int signed_count() { return 1 << 30; }

int main() {
  int status = 0;
  const size_type elements = size_type(1) << 28;
  new Destructible[unsigned_count()];
  if (requested != elements * 16 + 8) status |= 1;
  new Destructible[268435456];
  if (requested != elements * 16 + 8) status |= 2;
  new char[signed_count()];
  if (requested != size_type(1) << 30) status |= 4;
  new long[unsigned_count() * 2u];
  if (requested != elements * 16) status |= 8;
  return status;
}
