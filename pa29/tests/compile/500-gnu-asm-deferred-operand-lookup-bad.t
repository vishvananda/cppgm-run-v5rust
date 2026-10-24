inline void unused() {
  __asm__("outside_lifted_set" : : "r"(missing));
}
int main() { return 0; }
