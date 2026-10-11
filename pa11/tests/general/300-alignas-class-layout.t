struct alignas(16) ForwardAligned;
struct alignas(16) ForwardAligned { char value; };
struct alignas(32) DeclAligned { char value; };
struct alignas(long double) TypeAligned { char value; };

int aligned_call(int a, int b, int c, int d, int e, int f, int g) {
  DeclAligned local;
  local.value = 7;
  if ((unsigned long)&local % 32 != 0 || local.value != 7) return 0;
  return a + b + c + d + e + f + g;
}
int main() {
  if(alignof(ForwardAligned) != 16 || sizeof(ForwardAligned) != 16) {
    return 1;
  }
  if(alignof(DeclAligned) != 32 || sizeof(DeclAligned) != 32) {
    return 2;
  }
  if(alignof(TypeAligned) != alignof(long double) ||
     sizeof(TypeAligned) != alignof(long double)) {
    return 3;
  }
  DeclAligned local;
  local.value = 9;
  if ((unsigned long)&local % 32 != 0 || local.value != 9) return 4;
  if (aligned_call(1, 2, 3, 4, 5, 6, 7) != 28) return 5;
  if (local.value != 9) return 6;
  return 0;
}
