// cppgm++ array new of a host-built class from a braced-init-list: when a
// host constructor throws mid-list or in the default-constructed tail, the
// cppgm++ landing pads destroy the elements already built, in reverse, and
// free the storage before the exception reaches its handler.
extern int events[64];
extern int event_count;
extern int throw_at;
extern int allocations;
extern int releases;

struct Element
{
  int id;
  Element();
  Element(int v);
  ~Element();
};

Element* student_listed(int count, int first, int second, int third);
int host_check();
