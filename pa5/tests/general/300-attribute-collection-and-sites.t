// Attribute operands, appertainment and owner isolation share the production collector.
struct C {};
[[gnu::abi_tag("lead")]] alignas(C*) int [[gnu::vector_size(16)]] values[3] [[gnu::aligned(32)]];
using lanes [[gnu::ext_vector_type(4)]] = int;
namespace [[gnu::abi_tag("tag")]] N { int item; }
struct [[gnu::abi_tag("class")]] D {
  [[noreturn]] void f(int x [[vendor::unused]]) [[gnu::noinline]] noexcept;
  virtual [[vendor::unused]] ~D() noexcept {}
};
[[gnu::abi_tag("declaration")]] struct [[gnu::abi_tag("forward")]] F;
int symbol asm("external_symbol") __attribute__((weak));
void f() {
  [[course_attribute(argument{}()[(nested)])]] int x;
  [[gnu::used]] int y;
  [[vendor::ignored]] ;
}
