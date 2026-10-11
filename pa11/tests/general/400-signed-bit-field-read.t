struct Bits
{
  signed int value : 3;
};

int main()
{
  Bits bits = {-1};
  return bits.value;
}
