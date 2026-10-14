// expect exit 0
// Typed storage preserves requested alignment and the original initializer items.
alignas(32) int scalar = 7;
alignas(64) int zero;
alignas(32) int array[3] = {1, 2, 3};

struct alignas(32) Object
{
  char value;
};

int main()
{
  alignas(64) int local = 11;
  alignas(32) int elements[3] = {4, 5, 6};
  alignas(64) Object object = {9};
  static int block __attribute__((aligned(32))) = 13;
  if ((unsigned long)&scalar % 32 != 0) return 1;
  if ((unsigned long)&zero % 64 != 0) return 2;
  if ((unsigned long)&array % 32 != 0) return 3;
  if ((unsigned long)&local % 64 != 0) return 4;
  if ((unsigned long)&elements % 32 != 0) return 5;
  if ((unsigned long)&object % 64 != 0) return 6;
  if ((unsigned long)&block % 32 != 0) return 7;
  if (scalar != 7 || zero != 0 || array[2] != 3) return 8;
  if (local != 11 || elements[1] != 5 || object.value != 9 || block != 13) return 9;
  return 0;
}
