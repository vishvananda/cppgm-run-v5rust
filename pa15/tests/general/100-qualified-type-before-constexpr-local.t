namespace support {

typedef unsigned uint32_t;

}  // namespace support

int main()
{
  support::uint32_t constexpr constants[4] = {1, 2, 3, 4};
  return constants[2] != 3;
}
