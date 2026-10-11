struct Bits
{
  unsigned long words[4];

  Bits()
      : words()
  {
  }
};

int main()
{
  Bits bits;
  return bits.words[0] == 0 &&
         bits.words[1] == 0 &&
         bits.words[2] == 0 &&
         bits.words[3] == 0 ? 0 : 1;
}
