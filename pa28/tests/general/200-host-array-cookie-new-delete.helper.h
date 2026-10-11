// Array new and delete[] agree on the Itanium C++ ABI cookie (2.7) across
// objects: none for a trivially destructible element whose usual array
// deallocation function takes no size, eight bytes holding the count for a
// non-trivial destructor or a sized class deallocation function.
typedef unsigned long size_type;

extern int destroyed;
extern size_type sized_released;

struct Trivial {
  void* first;
  void* second;
};

struct Destructible {
  long value;
  Destructible() : value(7) {}
  ~Destructible() { ++destroyed; }
};

struct Sized {
  long value;
  static void* operator new[](size_type size);
  static void operator delete[](void* pointer, size_type size);
};

Trivial* host_make_trivial(unsigned count);
Destructible* host_make_destructible(unsigned count);
Sized* host_make_sized(unsigned count);
void host_drop_trivial(Trivial* array);
void host_drop_destructible(Destructible* array);
void host_drop_sized(Sized* array);
