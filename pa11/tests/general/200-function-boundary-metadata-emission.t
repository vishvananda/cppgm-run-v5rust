int pure() noexcept { return 7; }
void trap() { __builtin_unreachable(); }

int main() {
  return pure() + (int)__builtin_strlen("xy");
}
