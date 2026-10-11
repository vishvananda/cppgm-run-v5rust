enum Kind : unsigned { zero, one };
struct Bits
{
  unsigned small : 3;
  unsigned full : 32;
  unsigned long wide_small : 3;
  long signed_small : 3;
  bool flag : 1;
  Kind kind : 3;
};
int select(int) { return 1; }
int select(double) { return 2; }

int main()
{
  Bits bits = {0, 0, 0, -1, false, zero};
  Bits compound = {3, 0, 0, 0, false, zero};
  compound.small /= -2;
  return (bits.small - 1) / 2 == 0 &&
         (+bits.small - 1) < 0 && !(bits.small < -1) &&
         (~bits.small) == -1 && (bits.small << 1) == 0 &&
         (bits.full - 1) / 2 == 2147483647u &&
         (bits.wide_small - 1) / 2 == 0 &&
         select(+bits.wide_small) == 1 && bits.signed_small == -1 &&
         (+bits.flag - 1) < 0 && compound.small == 7 &&
         (+bits.kind - 1) / 2 == 2147483647u ? 0 : 1;
}
