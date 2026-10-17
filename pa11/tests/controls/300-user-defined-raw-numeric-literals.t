// N3485 2.14.8p3-p4: raw digits preserve spelling and defer range; expect exit 0
const char *operator ""_raw(const char *text) { return text; }

int main()
{
  const char *text = "0x00";
  if (0x00_raw != text || 0x00_raw != 0x00_raw)
    return 1;
  if (18446744073709551616_raw[19] != '6')
    return 2;
  if (1e999999999_raw[10] != '9' || 1e999999999_raw[11] != 0)
    return 3;
  if (1.25e-3_raw[5] != '-' || 077_raw[0] != '0')
    return 4;
  return 0;
}
