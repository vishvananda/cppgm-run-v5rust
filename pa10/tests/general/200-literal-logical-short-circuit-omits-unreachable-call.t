// Literal short-circuit operands must not retain an unreachable symbol
// reference in LowIR at -O0.

bool unavailable();

int main()
{
  (true || unavailable() ? (void)0 : (void)0);
  (false && unavailable() ? (void)0 : (void)0);
  return 0;
}
