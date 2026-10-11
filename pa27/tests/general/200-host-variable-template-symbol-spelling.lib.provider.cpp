#include "200-host-variable-template-symbol-spelling.helper.h"

int* host_width() { return &width<int>; }
int host_bump() { return ++store::count<char>; }
