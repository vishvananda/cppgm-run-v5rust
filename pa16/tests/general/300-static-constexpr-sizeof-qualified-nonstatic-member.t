template<unsigned long R>
struct chacha
{
  unsigned int keysetup_[8];
  static constexpr unsigned long state_size = sizeof(chacha::keysetup_);
};

int main()
{
  static_assert(chacha<20>::state_size == sizeof(unsigned int[8]), "size");
  return 0;
}
