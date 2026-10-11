// VALIDATION: compile-pass
// N3485 focus: 8.3.6 [dcl.fct.default], 9.3 [class.mfct]

struct Text {
  int take(char *out, unsigned long n, unsigned long pos = 0);
};

int Text::take(char *out, unsigned long n, unsigned long pos)
{
  out[0] = 'x';
  return (int)(n + pos);
}

int main()
{
  Text text;
  char out[4] = {};
  return text.take(out, 3) == 3 ? 0 : 1;
}
