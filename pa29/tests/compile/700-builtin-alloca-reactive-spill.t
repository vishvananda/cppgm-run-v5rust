// VALIDATION: compile-pass
// GNU/Clang hosted builtin focus: __builtin_alloca in a high-pressure lowering path

long clobber(long, long, long, long, long, long)
{
  return 0;
}

long use_alloca(long a, long b, long c, long d, long e,
                long f, long g, long h, long i, long j)
{
  long t1 = a + 1;
  long t2 = b + 2;
  long t3 = c + 3;
  long t4 = d + 4;
  long t5 = e + 5;
  long t6 = f + 6;
  long t7 = g + 7;
  long t8 = h + 8;
  long t9 = i + 9;
  long t10 = j + 10;
  long *p = static_cast<long *>(__builtin_alloca(16));
  *p = 5;
  long ignored = clobber(1, 2, 3, 4, 5, 6);
  return t1 + t2 + t3 + t4 + t5 + t6 + t7 + t8 + t9 + t10 + *p + ignored;
}

int main()
{
  return use_alloca(1, 2, 3, 4, 5, 6, 7, 8, 9, 18) == 123 ? 0 : 1;
}
