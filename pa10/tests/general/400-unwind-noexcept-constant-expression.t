// lowir.md unwind=no: noexcept(constant-expression) is non-throwing exactly
// when the expression's value is true (15.4p1, p12), however it is spelled.
int four_bytes(int x) noexcept(sizeof(int) == 4);
int eight_bytes(int x) noexcept(sizeof(int) == 8);
int nested(int x) noexcept(noexcept(four_bytes(0)));
int arithmetic(int x) noexcept(1 + 1 == 2);

int four_bytes(int x) noexcept(sizeof(int) == 4) { return x + 1; }
int eight_bytes(int x) noexcept(sizeof(int) == 8) { return x + 2; }
int nested(int x) noexcept(noexcept(four_bytes(0))) { return x + 3; }
int arithmetic(int x) noexcept(1 + 1 == 2) { return x + 4; }

int main()
{
  return four_bytes(0) + eight_bytes(0) + nested(0) + arithmetic(0) == 10 ? 0 : 1;
}
