#include "200-host-array-new-class-list-unwind.helper.h"
#include <stdlib.h>

int events[64];
int event_count = 0;
int throw_at = -1;
int allocations = 0;
int releases = 0;

static void note(int event) { if (event_count < 64) events[event_count++] = event; }

Element::Element() : id(100 + event_count) { note(id); if (id == throw_at) throw id; }
Element::Element(int v) : id(v) { note(v); if (v == throw_at) throw v; }
Element::~Element() { note(-id); }

void* operator new[](unsigned long size) { ++allocations; return malloc(size); }
void operator delete[](void* storage) noexcept { ++releases; free(storage); }

static bool same(const int* expected, int length)
{
  if (event_count != length) return false;
  for (int i = 0; i < length; ++i)
    if (events[i] != expected[i]) return false;
  return true;
}

int host_check()
{
  int status = 0;
  throw_at = 2;
  try { student_listed(4, 1, 2, 3); status |= 1; }
  catch (int thrown) { if (thrown != 2) status |= 2; }
  static const int clause[] = { 1, 2, -1 };
  if (!same(clause, 3) || allocations != 1 || releases != 1) status |= 4;
  event_count = 0;
  throw_at = 104;
  try { student_listed(6, 1, 2, 3); status |= 8; }
  catch (int thrown) { if (thrown != 104) status |= 16; }
  static const int tail[] = { 1, 2, 3, 103, 104, -103, -3, -2, -1 };
  if (!same(tail, 9) || allocations != 2 || releases != 2) status |= 32;
  return status;
}
