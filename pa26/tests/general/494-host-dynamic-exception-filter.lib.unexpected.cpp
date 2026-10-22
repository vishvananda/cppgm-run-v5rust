#include <exception>
static void replacement()
{
	throw 41;
}
extern "C" void install_replacement()
{
	std::set_unexpected(replacement);
}
