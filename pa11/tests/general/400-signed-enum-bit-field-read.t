enum Sign
{
  negative = -1,
  zero = 0
};

struct Bits
{
  Sign value : 2;
};

int main()
{
  Bits bits = {negative};
  return bits.value;
}
