struct Aligned
{
  alignas(8) char value;
};

int main()
{
  return alignof(Aligned) == 8 && sizeof(Aligned) == 8 ? 0 : 1;
}
