#include "200-host-itanium-class-layout.helper.h"

Base::Base() : value(1), tag(2) {}
Mark::Mark() : mark(5) {}
Holder::Holder() {}
EmptyDerived::EmptyDerived() : tail(9) {}

long host_signature() { return signature(); }

void host_fill(Mixed& mixed, Signs& signs, After& after)
{
  mixed.a = -2;
  mixed.b = true;
  mixed.c = 12345;
  signs.v = 2;
  signs.n = -3;
  signs.f = true;
  after.c = 'h';
  after.x = 1234;
  after.d = 'k';
}

int host_check(const Mixed& mixed, const Signs& signs, const After& after)
{
  return mixed.a == 3 && !mixed.b && mixed.c == -54321 && signs.v == 1 &&
    signs.n == 2 && !signs.f && after.c == 'o' && after.x == -77 &&
    after.d == 'u';
}

void host_assign_base(Base& target, const Base& source) { target = source; }

int host_read_mark(Holder& holder) { return holder.mark; }
