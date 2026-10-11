// VALIDATION: compile-pass
// N3485 focus: 5.19 [expr.const], 14.3.2 [temp.arg.nontype]

template<bool B>
struct Pick {
  char data[B ? 1 : 2];
};

int main()
{
  const bool enabled = true;
  return sizeof(Pick<enabled>) == 1 ? 0 : 1;
}
