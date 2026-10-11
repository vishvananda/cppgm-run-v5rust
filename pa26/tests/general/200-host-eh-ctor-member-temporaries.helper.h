// Output for differential programs: the PA25 no-host helper's
// cppgm_test_write_stdout (host-compiled), the same under g++ and cppgm++.
extern "C" long cppgm_test_write_stdout(const char* buf, unsigned long count);

static void out_text(const char* text)
{
  unsigned long n = 0;
  while(text[n] != 0)
    ++n;
  cppgm_test_write_stdout(text, n);
}

static void out_hex(unsigned long long value)
{
  char buf[19];
  buf[0] = ' ';
  for(int i = 0; i < 16; ++i)
  {
    unsigned digit = (unsigned)(value >> (60 - 4 * i)) & 15u;
    buf[1 + i] = (char)(digit < 10 ? '0' + digit : 'a' + digit - 10);
  }
  buf[17] = 0;
  out_text(buf);
}

static void out_line(const char* label, unsigned long long value)
{
  out_text(label);
  out_hex(value);
  out_text("\n");
}
