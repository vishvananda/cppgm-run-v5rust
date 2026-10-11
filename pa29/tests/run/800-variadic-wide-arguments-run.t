// SysV psABI 3.2.3, 3.5.7: an __int128 or a pointer to member function passed
// to `...`, constant ones too (an __int128 literal, a null member function
// pointer), is read back by __builtin_va_arg from two INTEGER eightbytes, in
// memory 16-aligned for __int128 and 8-aligned for a member function pointer
