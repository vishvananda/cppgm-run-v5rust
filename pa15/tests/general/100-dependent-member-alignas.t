template<int Alignment>
struct Aligned
{
  alignas(Alignment) char value;
};

int main()
{
  return alignof(Aligned<8>) == 8 && sizeof(Aligned<8>) == 8 ? 0 : 1;
}
