typedef __uint128_t U128;

unsigned long long local_high_digits_39()
{
  constexpr U128 digits_39 =
      static_cast<U128>(10000000000000000000ULL) *
      static_cast<U128>(10000000000000000000ULL);
  return static_cast<unsigned long long>(digits_39 >> 64);
}

int main()
{
  return local_high_digits_39() == 5421010862427522170ULL ? 0 : 1;
}
