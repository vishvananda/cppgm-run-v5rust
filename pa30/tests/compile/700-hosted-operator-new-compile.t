#include <new>
#include <cstddef>
#include <type_traits>
static_assert(std::is_same<decltype(::operator new(static_cast<std::size_t>(1))), void*>::value, "operator new -> void*");
static_assert(std::is_same<decltype(::operator new(static_cast<std::size_t>(1), static_cast<void*>(0))), void*>::value, "placement operator new -> void*");
