// expect exit 0
// Nonvirtual layout: POD/non-POD tails, empty collisions and packing precedence.
struct Empty {};
struct alignas(16) AlignedEmpty {};
struct Pod { int value; char tag; };
struct NonPod { NonPod(); int value; char tag; };
struct PodDerived : Pod { char extra; };
struct TailDerived : NonPod { char extra; short more; };
struct EmptyHolder { Empty elements[3]; };
struct EmptyDerived : Empty { EmptyHolder holder[2]; };
struct EmptyChain : Empty {};
struct EmptyCollision : EmptyChain { Empty member; };
struct __attribute__((packed)) PackedDerived : Pod { short extra; };
#pragma pack(push, 1)
struct CappedDerived : Pod { short extra; };
struct UncappedEmpty : AlignedEmpty { char member; };
#pragma pack(pop)
struct alignas(32) RequestedDerived : Pod { char extra; };

int main()
{
  if (sizeof(PodDerived) != 12 || sizeof(TailDerived) != 8) return 1;
  if (sizeof(EmptyDerived) != 7 || sizeof(EmptyCollision) != 2) return 2;
  if (sizeof(PackedDerived) != 12 || alignof(PackedDerived) != 4) return 3;
  if (sizeof(CappedDerived) != 10 || alignof(CappedDerived) != 1) return 4;
  if (sizeof(UncappedEmpty) != 16 || alignof(UncappedEmpty) != 16) return 5;
  if (sizeof(RequestedDerived) != 32 || alignof(RequestedDerived) != 32) return 6;
  return 0;
}
