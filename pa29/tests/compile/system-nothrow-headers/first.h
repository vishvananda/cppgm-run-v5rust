extern "C" int system_empty() throw();
extern "C" int system_plain() noexcept;
extern "C" int system_true() noexcept(true);
extern "C" int system_value() noexcept(1);
int cpp_nothrow() noexcept;
#include "nested.h"
