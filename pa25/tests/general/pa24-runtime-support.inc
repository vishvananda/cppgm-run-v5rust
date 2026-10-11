typedef unsigned char cppgm_u8;
typedef unsigned short cppgm_u16;
typedef unsigned int cppgm_u32;
typedef unsigned long long cppgm_u64;

extern "C" void * cppgm_test_alloc(unsigned long size);
extern "C" long cppgm_test_read_stdin(char * buf, unsigned long count);
extern "C" long cppgm_test_write_stdout(const char * buf, unsigned long count);

enum
{
  CPPGM_TEST_INBUF_CAP = 0x08000000,
  CPPGM_TEST_OUTBUF_CAP = 0x08000000
};

static char * cppgm_test_inbuf = 0;
static char * cppgm_test_in_cur = 0;
static char * cppgm_test_in_end = 0;
static char * cppgm_test_outbuf = 0;
static char * cppgm_test_out_cur = 0;
static char * cppgm_test_out_end = 0;
static int cppgm_test_io_failed = 0;

static cppgm_u64 cppgm_test_mask_for_bytes(int bytes)
{
  if(bytes >= 8)
  {
    return ~0ULL;
  }
  return (1ULL << (bytes * 8)) - 1ULL;
}

static cppgm_u64 cppgm_test_wrap_to_bytes(cppgm_u64 value, int bytes)
{
  return value & cppgm_test_mask_for_bytes(bytes);
}

static int cppgm_test_init_io()
{
  cppgm_test_io_failed = 0;
  if(cppgm_test_inbuf == 0)
  {
    cppgm_test_inbuf = (char *)cppgm_test_alloc(CPPGM_TEST_INBUF_CAP);
    cppgm_test_outbuf = (char *)cppgm_test_alloc(CPPGM_TEST_OUTBUF_CAP);
    if(cppgm_test_inbuf == 0 || cppgm_test_outbuf == 0)
    {
      cppgm_test_io_failed = 1;
      return 1;
    }
  }

  cppgm_test_in_cur = cppgm_test_inbuf;
  cppgm_test_in_end = cppgm_test_inbuf;
  cppgm_test_out_cur = cppgm_test_outbuf;
  cppgm_test_out_end = cppgm_test_outbuf + CPPGM_TEST_OUTBUF_CAP;

  for(;;)
  {
    cppgm_u64 used = (cppgm_u64)(cppgm_test_in_end - cppgm_test_inbuf);
    if(used == CPPGM_TEST_INBUF_CAP)
    {
      cppgm_test_io_failed = 1;
      return 1;
    }
    long step = cppgm_test_read_stdin(cppgm_test_in_end, CPPGM_TEST_INBUF_CAP - used);
    if(step < 0)
    {
      cppgm_test_io_failed = 1;
      return 1;
    }
    if(step == 0)
    {
      break;
    }
    cppgm_test_in_end += step;
  }
  return 0;
}

static char cppgm_test_input_underflow()
{
  cppgm_test_io_failed = 1;
  return '\0';
}

static char cppgm_test_output_overflow()
{
  cppgm_test_io_failed = 1;
  return '\0';
}

static int cppgm_test_flush_output()
{
  if(cppgm_test_io_failed)
  {
    return 1;
  }
  if(cppgm_test_write_stdout(cppgm_test_outbuf,
                             (unsigned long)(cppgm_test_out_cur - cppgm_test_outbuf)) < 0)
  {
    return 1;
  }
  return 0;
}

#define cppgm_test_take_char() \
  (cppgm_test_in_cur == cppgm_test_in_end ? cppgm_test_input_underflow() : *cppgm_test_in_cur++)

#define cppgm_test_peek_char() \
  (cppgm_test_in_cur == cppgm_test_in_end ? cppgm_test_input_underflow() : *cppgm_test_in_cur)

#define cppgm_test_out_char(ch) \
  (cppgm_test_out_cur == cppgm_test_out_end ? cppgm_test_output_overflow() : (*cppgm_test_out_cur++ = (ch)))

static void cppgm_test_out_unsigned_decimal(cppgm_u64 value)
{
  char scratch[32];
  int count = 0;
  if(value == 0)
  {
    cppgm_test_out_char('0');
    return;
  }
  while(value != 0)
  {
    scratch[count++] = (char)('0' + (value % 10ULL));
    value /= 10ULL;
  }
  while(count > 0)
  {
    cppgm_test_out_char(scratch[--count]);
  }
}

static int cppgm_test_is_negative(cppgm_u64 value, int bytes)
{
  value = cppgm_test_wrap_to_bytes(value, bytes);
  return ((value >> (bytes * 8 - 1)) & 1ULL) != 0ULL;
}

static cppgm_u64 cppgm_test_negate_raw(cppgm_u64 value, int bytes)
{
  return cppgm_test_wrap_to_bytes(0ULL - cppgm_test_wrap_to_bytes(value, bytes), bytes);
}

static void cppgm_test_out_signed_decimal_raw(cppgm_u64 value, int bytes)
{
  value = cppgm_test_wrap_to_bytes(value, bytes);
  if(cppgm_test_is_negative(value, bytes))
  {
    cppgm_test_out_char('-');
    cppgm_test_out_unsigned_decimal(cppgm_test_negate_raw(value, bytes));
  }
  else
  {
    cppgm_test_out_unsigned_decimal(value);
  }
}

static cppgm_u64 cppgm_test_take_count_line()
{
  cppgm_u64 value = 0;
  while(cppgm_test_peek_char() != '\n')
  {
    value = value * 10ULL + (cppgm_u64)(cppgm_test_take_char() - '0');
  }
  cppgm_test_take_char();
  return value;
}

static cppgm_u64 cppgm_test_take_unsigned_decimal()
{
  cppgm_u64 value = 0;
  while(cppgm_test_peek_char() >= '0' && cppgm_test_peek_char() <= '9')
  {
    value = value * 10ULL + (cppgm_u64)(cppgm_test_take_char() - '0');
  }
  return value;
}

static cppgm_u64 cppgm_test_take_signed_decimal_raw(int bytes)
{
  if(cppgm_test_peek_char() == '-')
  {
    cppgm_test_take_char();
    return cppgm_test_negate_raw(cppgm_test_take_unsigned_decimal(), bytes);
  }
  return cppgm_test_wrap_to_bytes(cppgm_test_take_unsigned_decimal(), bytes);
}

static cppgm_u8 cppgm_test_hex_value(char ch)
{
  if(ch >= '0' && ch <= '9')
  {
    return (cppgm_u8)(ch - '0');
  }
  return (cppgm_u8)(ch - 'a' + 10);
}

static cppgm_u8 cppgm_test_take_hex_byte()
{
  cppgm_u8 hi = cppgm_test_hex_value(cppgm_test_take_char());
  cppgm_u8 lo = cppgm_test_hex_value(cppgm_test_take_char());
  return (cppgm_u8)((hi << 4) | lo);
}

static cppgm_u64 cppgm_test_take_hex_le(int bytes)
{
  cppgm_u64 value = 0;
  for(int i = 0; i < bytes; ++i)
  {
    value |= ((cppgm_u64)cppgm_test_take_hex_byte()) << (i * 8);
  }
  return cppgm_test_wrap_to_bytes(value, bytes);
}

static cppgm_u64 cppgm_test_take_hex_be(int bytes)
{
  cppgm_u64 value = 0;
  for(int i = 0; i < bytes; ++i)
  {
    value = (value << 8) | (cppgm_u64)cppgm_test_take_hex_byte();
  }
  return cppgm_test_wrap_to_bytes(value, bytes);
}

static void cppgm_test_out_hex_digit(cppgm_u8 value)
{
  cppgm_test_out_char((char)(value < 10 ? ('0' + value) : ('a' + value - 10)));
}

static void cppgm_test_out_hex_byte(cppgm_u8 value)
{
  cppgm_test_out_hex_digit((cppgm_u8)(value >> 4));
  cppgm_test_out_hex_digit((cppgm_u8)(value & 0x0F));
}

static void cppgm_test_out_hex_le(cppgm_u64 value, int bytes)
{
  for(int i = 0; i < bytes; ++i)
  {
    cppgm_test_out_hex_byte((cppgm_u8)(value >> (i * 8)));
  }
}

static void cppgm_test_out_hex_be(cppgm_u64 value, int bytes)
{
  for(int i = bytes - 1; i >= 0; --i)
  {
    cppgm_test_out_hex_byte((cppgm_u8)(value >> (i * 8)));
  }
}

static int cppgm_test_signed_less(cppgm_u64 lhs, cppgm_u64 rhs, int bytes)
{
  lhs = cppgm_test_wrap_to_bytes(lhs, bytes);
  rhs = cppgm_test_wrap_to_bytes(rhs, bytes);
  const int lhs_negative = cppgm_test_is_negative(lhs, bytes);
  const int rhs_negative = cppgm_test_is_negative(rhs, bytes);
  if(lhs_negative != rhs_negative)
  {
    return lhs_negative;
  }
  if(!lhs_negative)
  {
    return lhs < rhs;
  }
  return cppgm_test_negate_raw(lhs, bytes) > cppgm_test_negate_raw(rhs, bytes);
}

static cppgm_u64 cppgm_test_signed_div(cppgm_u64 lhs, cppgm_u64 rhs, int bytes)
{
  lhs = cppgm_test_wrap_to_bytes(lhs, bytes);
  rhs = cppgm_test_wrap_to_bytes(rhs, bytes);
  const int lhs_negative = cppgm_test_is_negative(lhs, bytes);
  const int rhs_negative = cppgm_test_is_negative(rhs, bytes);
  cppgm_u64 lhs_abs = lhs_negative ? cppgm_test_negate_raw(lhs, bytes) : lhs;
  cppgm_u64 rhs_abs = rhs_negative ? cppgm_test_negate_raw(rhs, bytes) : rhs;
  cppgm_u64 quotient = lhs_abs / rhs_abs;
  if(lhs_negative != rhs_negative)
  {
    quotient = cppgm_test_negate_raw(quotient, bytes);
  }
  return cppgm_test_wrap_to_bytes(quotient, bytes);
}

static cppgm_u64 cppgm_test_signed_mod(cppgm_u64 lhs, cppgm_u64 rhs, int bytes)
{
  lhs = cppgm_test_wrap_to_bytes(lhs, bytes);
  rhs = cppgm_test_wrap_to_bytes(rhs, bytes);
  const int lhs_negative = cppgm_test_is_negative(lhs, bytes);
  const int rhs_negative = cppgm_test_is_negative(rhs, bytes);
  cppgm_u64 lhs_abs = lhs_negative ? cppgm_test_negate_raw(lhs, bytes) : lhs;
  cppgm_u64 rhs_abs = rhs_negative ? cppgm_test_negate_raw(rhs, bytes) : rhs;
  cppgm_u64 remainder = lhs_abs % rhs_abs;
  if(lhs_negative)
  {
    remainder = cppgm_test_negate_raw(remainder, bytes);
  }
  return cppgm_test_wrap_to_bytes(remainder, bytes);
}

static cppgm_u64 cppgm_test_signed_rshift(cppgm_u64 value, unsigned shift, int bytes)
{
  switch(bytes)
  {
  case 1:
    return (cppgm_u64)(cppgm_u8)((signed char)cppgm_test_wrap_to_bytes(value, 1) >> shift);
  case 2:
    return (cppgm_u64)(cppgm_u16)((short)cppgm_test_wrap_to_bytes(value, 2) >> shift);
  case 4:
    return (cppgm_u64)(cppgm_u32)((int)cppgm_test_wrap_to_bytes(value, 4) >> shift);
  default:
    return (cppgm_u64)((long long)cppgm_test_wrap_to_bytes(value, 8) >> shift);
  }
}

static float cppgm_test_take_f32()
{
  float value = 0.0f;
  cppgm_u8 * bytes = (cppgm_u8 *)&value;
  for(int i = 0; i < 4; ++i)
  {
    bytes[i] = cppgm_test_take_hex_byte();
  }
  return value;
}

static double cppgm_test_take_f64()
{
  double value = 0.0;
  cppgm_u8 * bytes = (cppgm_u8 *)&value;
  for(int i = 0; i < 8; ++i)
  {
    bytes[i] = cppgm_test_take_hex_byte();
  }
  return value;
}

static long double cppgm_test_take_f80()
{
  long double value = 0.0L;
  cppgm_u8 * bytes = (cppgm_u8 *)&value;
  for(unsigned long i = 0; i < sizeof(long double); ++i)
  {
    bytes[i] = 0;
  }
  for(int i = 0; i < 10; ++i)
  {
    bytes[i] = cppgm_test_take_hex_byte();
  }
  return value;
}

static void cppgm_test_out_f32(float value)
{
  const cppgm_u8 * bytes = (const cppgm_u8 *)&value;
  for(int i = 0; i < 4; ++i)
  {
    cppgm_test_out_hex_byte(bytes[i]);
  }
}

static void cppgm_test_out_f64(double value)
{
  const cppgm_u8 * bytes = (const cppgm_u8 *)&value;
  for(int i = 0; i < 8; ++i)
  {
    cppgm_test_out_hex_byte(bytes[i]);
  }
}

static void cppgm_test_out_f80(long double value)
{
  const cppgm_u8 * bytes = (const cppgm_u8 *)&value;
  for(int i = 0; i < 10; ++i)
  {
    cppgm_test_out_hex_byte(bytes[i]);
  }
}
