// N3485 3.4.2p2, 14.7.1p6: the associated namespaces of s::W<AS*> include s,
// W's, and those of AS, the class template specialization s::B<char,
// Arena<char>>; instantiating B, which overload resolution may do, looks tag
// up by argument-dependent lookup too, and operator!= is found in s (libc++'s
// allocator-aware containers)
