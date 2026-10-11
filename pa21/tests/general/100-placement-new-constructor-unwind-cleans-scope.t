void* operator new(unsigned long, void* p) noexcept { return p; }

struct guard { guard(); ~guard(); };
struct item { item(); };
long long storage;

void build() {
  guard cleanup;
  ::new(&storage) item;
}
