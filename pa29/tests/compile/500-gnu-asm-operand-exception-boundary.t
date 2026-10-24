int input();
void throwing_operand() noexcept {
  asm("" : : "r"(input()));
}
void quiet() noexcept {
  asm("mfence" ::: "memory");
}
struct Temporary { Temporary(); ~Temporary(); };
int read(Temporary const&);
void temporary_operand() noexcept {
  asm("" : : "r"(read(Temporary())));
}
