struct C {
  union { union { int a; long b; }; char c; };
  char d;
};
static_assert(sizeof(C) == 16, "single union storage");
static_assert(alignof(C) == 8, "union alignment");
int sizes[sizeof(C)];
