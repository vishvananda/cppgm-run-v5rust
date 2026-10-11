#include "800-decltype-new-delete-host-a-run.helper.h"
long a_side()
{
	int v = 0, w = 0;
	release(&v);
	release_array(&w);
	return (long)counter(1) + (long)counter_init(1) + (long)counter_global(1) + v + w;
}
