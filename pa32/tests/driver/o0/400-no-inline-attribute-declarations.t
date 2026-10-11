// lowir.md no_inline (compared here, in the -O0 driver lane, because PA27
// owns the attribute): a function states no_inline=yes when any declaration of
// it carries GNU noinline, before the declaration or after its declarator-id;
// a definition without the attribute keeps it.
__attribute__((noinline)) int declared_first(int x);
int declared_first(int x) { return x + 1; }
int after_declarator(int x) __attribute__((noinline));
int after_declarator(int x) { return x + 2; }
struct Box
{
  int value;
  __attribute__((noinline)) int member() const { return value + 3; }
};
int ordinary(int x) { return x + 4; }

int main()
{
  Box box;
  box.value = 0;
  return declared_first(0) + after_declarator(0) + box.member() + ordinary(0) == 10 ? 0 : 1;
}
