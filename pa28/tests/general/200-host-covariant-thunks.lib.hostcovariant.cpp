#include "200-host-covariant-thunks.helper.h"

int host_root(Root& value, Root* expected) { return value.self() == expected; }
int host_near(Near& value, Near* expected) { return value.self() == expected; }
