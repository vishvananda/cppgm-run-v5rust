// expect exit 0
struct Mixed { long a : 3; bool b : 1; };
struct Narrow { int lead; unsigned a : 3; char tail; };
struct Zero { unsigned : 0; };
struct Wide { unsigned char : 12; char tail; };
struct Base { Base(); long a : 3; };
struct Derived : Base { char tail; };
#pragma pack(push, 1)
struct Split { unsigned a : 4; unsigned long b : 64; };
#pragma pack(pop)
int main() {
  return sizeof(Mixed) == 8 && alignof(Mixed) == 8 &&
         sizeof(Narrow) == 8 && sizeof(Zero) == 1 && alignof(Zero) == 1 &&
         sizeof(Wide) == 3 && alignof(Wide) == 1 &&
         sizeof(Base) == 8 && sizeof(Derived) == 8 &&
         sizeof(Split) == 9 && alignof(Split) == 1 ? 0 : 1;
}
