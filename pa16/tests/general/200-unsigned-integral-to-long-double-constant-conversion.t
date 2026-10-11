// VALIDATION: compile-pass
// Converting a high-bit unsigned value to long double does not reinterpret it
// as a signed payload.

static_assert(18446744073709551615ULL > 1.0e19L,
              "unsigned conversion preserves its magnitude");

int main()
{
  return 0;
}
