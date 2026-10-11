#pragma pack(push, 1)
struct B {
  char a;
  int b;
};
#pragma pack(pop)

struct C {
  char a;
  int b;
};

int main()
{
  return sizeof(B) == 5 && sizeof(C) == 8 ? 0 : 1;
}
