// N3485 5.4: C-style reinterpret-like enumeration-to-pointer conversion.
enum { zero };
int *f() { return (int *)zero; }
