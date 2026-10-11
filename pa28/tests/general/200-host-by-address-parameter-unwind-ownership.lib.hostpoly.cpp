#include "200-host-by-address-parameter-unwind-ownership.helper.h"

namespace {
struct Both : First, Second {
  int first(Tracked value) { return value.id; }
  int second(Tracked value) { return value.self == &value ? value.id + 1 : -1; }
};
}

int host_throw(Tracked value) {
  if (value.self == &value) throw value.id;
  return 0;
}

Second* host_second() { static Both both; return &both; }

int host_calls_student() {
  int status = 0;
  Tracked original(30);
  const int destroyed_before = destroyed;
  try { student_throw(original); status |= 1; }
  catch (int thrown) { if (thrown != 30) status |= 2; }
  if (live != 1 || destroyed - destroyed_before != 1) status |= 4;
  int result = student_second()->second(original);
  if (result != 31 || live != 1 || destroyed - destroyed_before != 2) status |= 8;
  return status;
}
