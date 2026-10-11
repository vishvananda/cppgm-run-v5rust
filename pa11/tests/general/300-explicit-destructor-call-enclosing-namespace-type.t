#include "300-explicit-destructor-call-enclosing-namespace-type.h"

void destroy(ns::json::detail::holder * h)
{
  h->~holder();
}

int main()
{
  return 0;
}
