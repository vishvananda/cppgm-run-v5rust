// Array cookies follow the Itanium C++ ABI (2.7): none when the element type
// has a trivial destructor and its usual array deallocation function takes no
// size; otherwise max(sizeof(size_t), alignof(T)) bytes precede the array,
// the element count in the last eight of them, and delete[] passes the whole
// allocation to a sized deallocation function.
typedef unsigned long size_type;

size_type requested = 0;
char* allocated = 0;
size_type sized_released = 0;

char storage[512];

void* operator new[](size_type size) {
  requested = size;
  allocated = storage + 64;
  return allocated;
}

void operator delete[](void*) noexcept {}

struct Trivial {
  long first;
  long second;
};

struct Destructible {
  long value;
  ~Destructible() {}
};

struct alignas(32) Aligned {
  long value;
  ~Aligned() {}
};

struct Sized {
  long value;
  static void* operator new[](size_type size) {
    requested = size;
    allocated = storage + 64;
    return allocated;
  }
  static void operator delete[](void*, size_type size) { sized_released = size; }
};

size_type stored_count(const void* array) {
  return *(reinterpret_cast<const size_type*>(array) - 1);
}

int main() {
  int status = 0;
  Trivial* trivial = new Trivial[3];
  if (requested != 48 || reinterpret_cast<char*>(trivial) != allocated)
    status |= 1;
  delete[] trivial;
  Destructible* destructible = new Destructible[3];
  if (requested != 32 || reinterpret_cast<char*>(destructible) != allocated + 8 ||
    stored_count(destructible) != 3) status |= 2;
  delete[] destructible;
  Aligned* aligned = new Aligned[3];
  if (requested != 128 || reinterpret_cast<char*>(aligned) != allocated + 32 ||
    stored_count(aligned) != 3) status |= 4;
  delete[] aligned;
  Sized* sized = new Sized[3];
  if (requested != 32 || reinterpret_cast<char*>(sized) != allocated + 8 ||
    stored_count(sized) != 3) status |= 8;
  delete[] sized;
  if (sized_released != 32) status |= 16;
  return status;
}
