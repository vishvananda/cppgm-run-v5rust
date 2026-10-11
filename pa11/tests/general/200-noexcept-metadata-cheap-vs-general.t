int cheap() noexcept(true && !false) { return 1; }
int general() noexcept(1) { return 2; }

int main() {
  return cheap() + general();
}
