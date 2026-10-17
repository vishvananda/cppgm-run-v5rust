// N3485 2.14.8p3-p4: cooked signatures precede raw signatures; expect exit 0
int operator ""_tag(const char *) { return 100; }
unsigned long long operator ""_tag(unsigned long long value) { return value; }
long double operator ""_tag(long double value) { return value; }
int value;
int &operator ""_ref(unsigned long long) { return value; }

int main()
{
  if (35_tag != 35ULL || 077_tag != 63ULL || 0xffffffff_tag != 4294967295ULL)
    return 1;
  if (9223372036854775808_tag != 9223372036854775808ULL)
    return 2;
  if (18446744073709551615_tag != 18446744073709551615ULL)
    return 3;
  if (1.0000000000000000001_tag == 1.0L || 1e4000_tag <= 0.0L)
    return 4;
  if (1e-999999999_tag != 0.0L)
    return 5;
  0_ref = 7;
  3_ref;
  if (4_ref != 7)
    return 6;
  return 0;
}
