struct __attribute__((packed)) packed { char a; int b; };
static_assert(sizeof(packed) == 5, "");
