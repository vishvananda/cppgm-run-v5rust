#include "200-host-inline-local-static-symbol-spelling.helper.h"

int host_calls()
{
  return counter() + Box().bump() + local_class_counter();
}
