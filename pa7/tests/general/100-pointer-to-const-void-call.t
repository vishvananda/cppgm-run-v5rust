// HHC-165
unsigned long __loadword(const void*);

unsigned long f(const char* p) { return __loadword(p); }
