// An invalid array new count in cppgm++ code obtains no storage and throws
// std::bad_array_new_length through __cxa_throw_bad_array_new_length; the
// host catches it after the cppgm++ frame's locals were destroyed.
typedef unsigned long size_type;

extern int live;
extern int allocations;

struct Guard {
  Guard() { ++live; }
  ~Guard() { --live; }
};

struct Destructible {
  long value;
  Destructible() : value(7) {}
  ~Destructible() {}
};

int student_signed(int count);
int student_long(long count);
int student_sized(size_type count);
int student_class(long count);
