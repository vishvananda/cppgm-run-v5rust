struct Bits
{
  unsigned int value : 1;
};

int main()
{
  Bits bits = {0};
  unsigned int *pointer = &bits.value;
  return pointer != 0;
}
