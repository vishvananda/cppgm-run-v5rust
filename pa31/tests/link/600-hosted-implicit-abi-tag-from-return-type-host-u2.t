// GCC abi_tag, PA29 and PA31 README (host names keep ABI tags): a function
// returning std::string (std::__cxx11::basic_string, tagged cxx11) is named
// _Z8get_in_bB5cxx11v, the implicit tag included, so a cppgm++ caller links
// with a host-built definition
