#include "200-host-inline-lambda-closure-weak-link.helper.h"

int host_ticket() { return next_ticket(); }
int* host_tag() { return ticket_tag(); }
