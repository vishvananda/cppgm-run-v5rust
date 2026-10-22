#include <exception>
#include <unistd.h>
static int cleanups;
extern "C" void record_cleanup()
{
	++cleanups;
}
static void unexpected()
{
	if (cleanups == 1)
	{
		const char message[] = "empty unexpected after cleanup\n";
		if (write(1, message, sizeof(message) - 1) == sizeof(message) - 1)
			_exit(0);
	}
	_exit(2);
}
extern "C" void install_empty_handler()
{
	std::set_unexpected(unexpected);
}
