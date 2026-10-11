struct Bits
{
  unsigned int first : 1;
  unsigned int second : 2;
};

int main()
{
  Bits bits = {1, 1};
  ++bits.second;
  bits.second++;
  return bits.first == 1 && bits.second == 3 ? 0 : 1;
}
