namespace flags {
enum Fmt {
  fmt = 1
};

enum Mode {
  in = 1,
  out = 2
};

Fmt operator|(Fmt, Fmt)
{
  return fmt;
}

Mode operator|(Mode, Mode)
{
  return out;
}
}

int accept(flags::Mode)
{
  return 0;
}

int accept(int)
{
  return 1;
}

int main()
{
  flags::Mode mode = flags::in;
  return accept(mode | flags::out);
}
