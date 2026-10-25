#include <first.h>
extern "C" int system_empty();
extern "C" int system_plain();
extern "C" int system_true();
extern "C" int system_nested();
extern "C" int system_empty() { return 1; }
extern "C" int system_plain() { return 2; }
extern "C" int system_true() { return 3; }
extern "C" int system_nested() { return 4; }
static_assert(noexcept(system_empty()), "throw specification retained");
static_assert(noexcept(system_plain()), "noexcept retained");
static_assert(noexcept(system_true()), "true retained");
static_assert(noexcept(system_nested()), "relative inclusion and macro provenance");
int call_all() noexcept {
  return system_empty() + system_plain() + system_true() + system_nested();
}
