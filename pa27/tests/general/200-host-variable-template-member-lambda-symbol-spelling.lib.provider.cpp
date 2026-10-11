#include "200-host-variable-template-member-lambda-symbol-spelling.helper.h"

int host_next() { return next_id<int>(); }
int host_count() { return ++table::count<int>; }
