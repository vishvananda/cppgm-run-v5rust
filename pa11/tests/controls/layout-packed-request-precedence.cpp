// expect exit 0
// GNU packed honors a member request; pragma pack caps that request.
struct __attribute__((packed)) Packed
{
  char first;
  int value __attribute__((aligned(8)));
};

#pragma pack(push, 2)
struct Capped
{
  char first;
  alignas(16) int value;
};
#pragma pack(pop)

struct NonPod
{
  char first;
  int value;
  NonPod() : first(1), value(2) {}
};

struct __attribute__((packed)) Outer
{
  char first;
  NonPod values[2];
};

struct VariadicAssignment
{
  char first;
  int value;
  VariadicAssignment& operator=(const VariadicAssignment&, ...);
};

struct __attribute__((packed)) VariadicOuter
{
  char first;
  VariadicAssignment value;
};

int main()
{
  if (sizeof(Packed) != 16 || alignof(Packed) != 8) return 1;
  if (sizeof(Capped) != 6 || alignof(Capped) != 2) return 2;
  if (sizeof(Outer) != 20 || alignof(Outer) != 4) return 3;
  if (sizeof(VariadicOuter) != 9 || alignof(VariadicOuter) != 1) return 5;
  Packed packed = {1, 7};
  Capped capped = {2, 9};
  if (packed.value != 7 || capped.value != 9) return 4;
  return 0;
}
