// A qualified constant in an included class must be loaded on first use.
#include "500-lazy-qualified-static-value-demand.h"

typedef advance::apply<number<1> >::type result;
int main() { return 0; }
