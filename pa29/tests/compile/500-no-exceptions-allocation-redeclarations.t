// -fno-exceptions keeps the first allocation/deallocation exception facts.
void* operator new(unsigned long);
void* operator new(unsigned long) noexcept;
void operator delete(void*) noexcept;
void operator delete(void*) noexcept(false);
struct Store {
  static void* operator new(unsigned long) noexcept;
  static void operator delete(void*) noexcept(false);
};
void* Store::operator new(unsigned long) noexcept(false) { return 0; }
void Store::operator delete(void*) noexcept {}
static_assert(!noexcept(::operator new(1)), "first global allocation");
static_assert(noexcept(::operator delete(nullptr)), "first global deletion");
static_assert(noexcept(Store::operator new(1)), "first class allocation");
static_assert(!noexcept(Store::operator delete(nullptr)), "first class deletion");
