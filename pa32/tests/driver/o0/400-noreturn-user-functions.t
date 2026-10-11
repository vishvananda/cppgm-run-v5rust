// lowir.md return=noreturn (compared here, in the -O0 driver lane, because
// PA29 owns the attribute): a function declaration, definition or call
// signature states return=noreturn when any declaration of the function
// carries [[noreturn]] or GNU noreturn (7.6.3): a free function, by its first
// declaration or (GNU noreturn) a later one, a member function, a function
// template's specializations and a virtual call; not an explicit
// specialization declared without it, and not a call through a pointer (the
// attribute appertains to a function, not to its type).
[[noreturn]] void stop(int code) { throw code; }
__attribute__((noreturn)) void halt(int code);
void halt(int code) { throw code + 1; }
void later(int code);
void later(int code) __attribute__((noreturn));
void later(int code) { throw code + 2; }
struct Guard
{
  [[noreturn]] void fail() const { throw 3; }
  [[noreturn]] virtual void abandon() const { throw 4; }
  virtual ~Guard() {}
};
template<class T> [[noreturn]] void raise_code(T code) { throw static_cast<int>(code); }
template<> void raise_code<char>(char) {}

int main(int argc, char**)
{
  Guard guard;
  const Guard& view = guard;
  void (*through)(int) = stop;
  raise_code('x');
  try
  {
    if (argc > 7) through(7);
    if (argc > 6) stop(6);
    if (argc > 5) halt(5);
    if (argc > 4) later(4);
    if (argc > 3) guard.fail();
    if (argc > 2) view.abandon();
    if (argc > 1) raise_code(1L);
  }
  catch (int code)
  {
    return code;
  }
  return 0;
}
