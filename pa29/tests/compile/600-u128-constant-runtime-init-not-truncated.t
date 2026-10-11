typedef __uint128_t U128;

static const U128 digits_39 =
    static_cast<U128>(10000000000000000000ULL) *
    static_cast<U128>(10000000000000000000ULL);

unsigned long long high_digits_39()
{
  return static_cast<unsigned long long>(digits_39 >> 64);
}

int main()
{
  return high_digits_39() == 5421010862427522170ULL ? 0 : 1;
}
