// Hosted headers use vector_size typedefs in inline wrappers.  Retain the
// vector byte width instead of silently treating the alias as its scalar lane.
typedef float vector4f
    __attribute__((__vector_size__(16), __may_alias__));

extern __inline __attribute__((__gnu_inline__, __always_inline__))
vector4f wrapper_zero()
{
  return __extension__ (vector4f){0, 0, 0, 0};
}

static_assert(sizeof(vector4f) == 16, "vector byte width");
int main() { return 0; }
