#pragma once

#include <cstddef>
#include <string>

namespace text_intern {

typedef const std::string * Atom;

Atom intern(const std::string & text);
Atom intern(const char * data, std::size_t length);
Atom find(const std::string & text);
std::size_t atom_count();
std::size_t storage_bytes();

}  // namespace text_intern
