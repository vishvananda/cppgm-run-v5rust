template <unsigned long Bits>
struct uint_for {
  typedef unsigned int fast;
};

template <unsigned long Bits, typename uint_for<Bits>::fast Poly>
struct crc_kind {
};

template <unsigned long Bits, typename uint_for<Bits>::fast Poly>
int run_crc() {
  crc_kind<Bits, Poly> value;
  (void)&value;
  return Poly == 7 ? 0 : 1;
}

int main() {
  return run_crc<32, 7>();
}
