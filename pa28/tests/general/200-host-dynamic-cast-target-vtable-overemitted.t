// PA31 README (demand-driven emission: only definitions the unit's code needs)
// + Itanium ABI 5.2.3: dynamic_cast to M<char16_t>* needs M<char16_t>'s
// type_info only; its vtable (naming the never-defined M<char16_t>::get) must
// not be emitted, so the program links
