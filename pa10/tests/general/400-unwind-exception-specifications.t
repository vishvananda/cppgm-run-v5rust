// lowir.md unwind=no: a function declaration or definition states unwind=no
// exactly when the function is non-throwing (15.4p12): noexcept,
// noexcept(true) and throw() do; noexcept(false), a
// dynamic-exception-specification naming a type and no specification do not.
int plain(int x);
int bare_noexcept(int x) noexcept;
int true_noexcept(int x) noexcept(true);
int empty_throw(int x) throw();
int false_noexcept(int x) noexcept(false);
int typed_throw(int x) throw(int);

int bare_noexcept(int x) noexcept { return x + 1; }
int true_noexcept(int x) noexcept(true) { return x + 2; }
int empty_throw(int x) throw() { return x + 3; }
int false_noexcept(int x) noexcept(false) { return x + 4; }
int typed_throw(int x) throw(int) { return x + 5; }
int plain(int x) { return x + 6; }

int main()
{
  return bare_noexcept(0) + true_noexcept(0) + empty_throw(0) + false_noexcept(0) +
    typed_throw(0) + plain(0) == 21 ? 0 : 1;
}
