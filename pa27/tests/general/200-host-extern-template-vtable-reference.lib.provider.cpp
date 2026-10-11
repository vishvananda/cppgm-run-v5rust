#include "200-host-extern-template-vtable-reference.helper.h"

template struct Facet<int>;

int host_read(const Facet<int> *value) { return value->read(); }
