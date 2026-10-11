// VALIDATION: compile-pass
// N3485 focus: 9.2 [class.mem], 9.5 [class.union]

struct Holder
{
  union
  {
    int i;
    char bytes[8];
  } value;
  int after;
};

int main()
{
  return sizeof(Holder) == 12 && sizeof(((Holder*)0)->value) == 8 ? 0 : 1;
}
