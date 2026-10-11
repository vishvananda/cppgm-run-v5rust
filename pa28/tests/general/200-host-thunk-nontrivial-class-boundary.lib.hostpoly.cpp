#include "200-host-thunk-nontrivial-class-boundary.helper.h"

Value Both::first(Value value) { return Value(value.self == &value ? value.n + 1 : -1); }
Value Both::second(Value value) { return Value(value.self == &value ? value.n + 2 : -1); }

Second* host_second() { static Both both; return &both; }

int host_calls_student(Second* second) {
  Value argument(30);
  Value result = second->second(argument);
  return result.n == 32 && result.self == &result ? 0 : 4;
}
